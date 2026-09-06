use std::{
    env,
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
};
use toml::Table;

const ASSET_PATH_VAR: &str = "BEVY_ASSET_PATH";

fn locate_project_workspace() -> Option<PathBuf> {
    // NOTE: Method I was doing before

    // let output = Command::new(std::env!("CARGO")).args(["locate-project", "--workspace"]).output().ok()?;
    // let output: Value = serde_json::from_slice(output.stdout.as_slice()).ok()?;

    // let workspace = output.get("root")?.as_str()?;
    // let workspace_path = std::path::PathBuf::from(workspace);
    // let workspace_path = workspace_path.parent().unwrap().to_path_buf();

    // cargo_emit::warning!("Workspace root found: {:?}", workspace);

    // return Some(workspace_path);

    //  NOTE: Pwd workaround that is kind of messy

    std::env::current_dir().ok()
}

fn load_embedded_assets_toml() -> Option<Table> {
    let workspace_path = locate_project_workspace()?;

    let mut buf: String = String::new();
    let mut f = std::fs::File::open(workspace_path.join("EmbeddedAssets.toml")).ok()?;
    let _ = std::fs::File::read_to_string(&mut f, &mut buf).ok()?;
    let table = buf.parse::<Table>().ok()?;

    // cargo_emit::warning!("{:?}", table);
    Some(table)
}

fn get_vector_string(table: &Table, key: String) -> Vec<String> {
    let mut out = vec![];

    let temp = table.get(&key);
    let Some(temp) = temp else {
        return out;
    };
    let temp = temp.as_array();
    let Some(temp) = temp else {
        return out;
    };

    out = temp.iter().map(|x| x.as_str().unwrap().to_string()).collect();
    cargo_emit::warning!("{}: {:?}", key, out);

    return out;
}

fn main() {
    cargo_emit::rerun_if_env_changed!(ASSET_PATH_VAR);

    let table = load_embedded_assets_toml();
    let mut ignore_extensions: Vec<String> = vec![];
    let mut ignore_dirs: Vec<String> = vec![];
    if let Some(table) = table {
        cargo_emit::warning!("Found EmbeddedAssets.toml");
        ignore_extensions = get_vector_string(&table, "ignore_extensions".into());
        ignore_dirs = get_vector_string(&table, "ignore_dirs".into());
    }

    // Check if env variable is set for the assets folder
    if let Some(dir) = env::var(ASSET_PATH_VAR)
        .ok()
        .map(|v| Path::new(&v).to_path_buf())
        .and_then(|path| {
            if path.exists() {
                Some(path)
            } else {
                cargo_emit::warning!("${} points to an unknown folder: {}", ASSET_PATH_VAR, path.to_string_lossy());
                None
            }
        })
        // Otherwise, search for the target folder and look for an assets folder next to it
        .or_else(|| {
            env::var("OUT_DIR")
                .ok()
                .map(|v| Path::new(&v).to_path_buf())
                .and_then(|path| {
                    for ancestor in path.ancestors() {
                        if let Some(last) = ancestor.file_name()
                            && last == "target"
                        {
                            return ancestor.parent().map(|parent| {
                                let imported_dir = parent.join("imported_assets");
                                if imported_dir.exists() { imported_dir.join("Default") } else { parent.join("assets") }
                            });
                        }
                    }
                    None
                })
                .and_then(|path| {
                    if path.exists() {
                        Some(path)
                    } else {
                        cargo_emit::warning!("Could not find asset folder from Cargo build directory");
                        None
                    }
                })
        })
    {
        cargo_emit::rerun_if_changed!(dir.to_string_lossy());
        cargo_emit::warning!("Asset folder found: {}", dir.to_string_lossy());

        let out_dir = env::var_os("OUT_DIR").unwrap();
        let dest_path = Path::new(&out_dir).join("include_all_assets.rs");

        let mut file = File::create(dest_path).unwrap();
        file.write_all(
            "/// Generated function that will embed all assets.
#[allow(unused_variables, unused_qualifications, clippy::non_ascii_literal)]
fn include_all_assets(mut registry: impl EmbeddedRegistry){\n"
                .as_ref(),
        )
        .unwrap();

        let building_for_not_windows = std::env::var("CARGO_CFG_TARGET_OS").is_ok_and(|v| v != "windows");

        visit_dirs(&dir, &ignore_extensions, &ignore_dirs)
            .iter()
            .map(|path| (path, path.strip_prefix(&dir).unwrap()))
            .for_each(|(fullpath, path)| {
                let mut path = path.to_string_lossy().to_string();
                if building_for_not_windows {
                    // replace paths with forward slash in case we're building from windows
                    path = path.replace(std::path::MAIN_SEPARATOR, "/");
                }
                cargo_emit::rerun_if_changed!(fullpath.to_string_lossy());
                file.write_all(
                    format!(
                        r#"    registry.insert_included_asset({:?}, include_bytes!({:?}));
"#,
                        path,
                        fullpath.to_string_lossy()
                    )
                    .as_ref(),
                )
                .unwrap();
            });

        file.write_all("}".as_ref()).unwrap();
    } else if std::env::var("DOCS_RS").is_ok() {
        let out_dir = env::var_os("OUT_DIR").unwrap();
        let dest_path = Path::new(&out_dir).join("include_all_assets.rs");

        let mut file = File::create(dest_path).unwrap();
        file.write_all(
            "/// Generated function that will embed all assets.
#[allow(unused_variables, unused_qualifications, clippy::non_ascii_literal)]
fn include_all_assets(registry: impl EmbeddedRegistry){}"
                .as_ref(),
        )
        .unwrap();
    } else {
        cargo_emit::warning!("Could not find asset folder, please specify its path with ${}", ASSET_PATH_VAR);
        panic!("No asset folder found");
    }
}

fn visit_dirs(dir: &Path, ignore_extensions: &Vec<String>, ignore_dirs: &Vec<String>) -> Vec<PathBuf> {
    let mut collected = vec![];
    if dir.is_dir() {
        // Do a prepass to check directory for an ignore file.
        // .bevyignore
        let mut entries = fs::read_dir(dir).unwrap();
        if entries.any(|x| {
            let entry = x.unwrap();
            let path = entry.path();
            path.ends_with(".bevyignore")
        }) {
            cargo_emit::warning!("Found .bevyignore at {:?}", dir);
            return collected;
        }

        // Add contents to the collected files only if there's no ignore file.
        for entry in fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();

            let end = path.file_name();
            let extension = path.extension();

            if extension.is_some() {
                if ignore_extensions.contains(&extension.unwrap().to_str().unwrap().to_string()) {
                    cargo_emit::warning!("Ignoring file: {:?}", path);
                    continue;
                }
            }

            if path.is_dir() {
                if ignore_dirs.contains(&end.unwrap().to_str().unwrap().to_string()) {
                    cargo_emit::warning!("Ignoring directory: {:?}", path);
                    continue;
                }
            }

            if path.is_dir() {
                collected.append(&mut visit_dirs(&path, &ignore_extensions, &ignore_dirs));
            } else {
                collected.push(path);
            }
        }
    }
    collected
}
