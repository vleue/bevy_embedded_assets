use std::{
    env,
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
};

const ASSET_PATH_VAR: &str = "BEVY_ASSET_PATH";

fn main() {
    cargo_emit::rerun_if_env_changed!(ASSET_PATH_VAR);

    // Check if env variable is set for the assets folder
    if let Some(dir) = env::var(ASSET_PATH_VAR)
        .ok()
        .map(|v| Path::new(&v).to_path_buf())
        .and_then(|path| {
            if path.exists() {
                Some(path)
            } else {
                cargo_emit::warning!(
                    "${} points to an unknown folder: {}",
                    ASSET_PATH_VAR,
                    path.to_string_lossy()
                );
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
                        if let Some(last) = ancestor.file_name() {
                            if last == "target" {
                                return ancestor.parent().map(|parent| {
                                    let imported_dir = parent.join("imported_assets");
                                    if imported_dir.exists() {
                                        imported_dir.join("Default")
                                    } else {
                                        parent.join("assets")
                                    }
                                });
                            }
                        }
                    }
                    None
                })
                .and_then(|path| {
                    if path.exists() {
                        Some(path)
                    } else {
                        cargo_emit::warning!(
                            "Could not find asset folder from Cargo build directory"
                        );
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

        let building_for_not_windows =
            std::env::var("CARGO_CFG_TARGET_OS").is_ok_and(|v| v != "windows");

        visit_dirs(&dir)
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
        cargo_emit::warning!(
            "Could not find asset folder, please specify its path with ${}",
            ASSET_PATH_VAR
        );
        panic!("No asset folder found");
    }
}

fn visit_dirs(dir: &Path) -> Vec<PathBuf> {
    let mut collected = vec![];
    if dir.is_dir() {
        for entry in fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_dir() {
                collected.append(&mut visit_dirs(&path));
            } else {
                collected.push(path);
            }
        }
    }
    collected
}
