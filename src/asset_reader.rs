use std::{
    io::Read,
    path::{Path, PathBuf},
    pin::Pin,
    task::Poll,
};

use bevy::{
    asset::io::{AssetReader, AssetReaderError, PathStream, Reader},
    utils::HashMap,
};
use futures_io::AsyncRead;
use futures_lite::Stream;

use crate::{include_all_assets, EmbeddedRegistry};

pub(crate) struct EmbeddedAssetReader {
    loaded: HashMap<&'static Path, &'static [u8]>,
    fallback: Option<Box<dyn AssetReader>>,
}

impl std::fmt::Debug for EmbeddedAssetReader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EmbeddedAssetReader")
            .finish_non_exhaustive()
    }
}

impl Default for EmbeddedAssetReader {
    fn default() -> Self {
        Self::new()
    }
}

impl EmbeddedRegistry for &mut EmbeddedAssetReader {
    fn insert_included_asset(&mut self, name: &'static str, bytes: &'static [u8]) {
        self.add_asset(Path::new(name), bytes);
    }
}

impl EmbeddedAssetReader {
    /// Create an empty [`EmbeddedAssetReader`].
    #[must_use]
    pub fn new() -> Self {
        Self {
            loaded: HashMap::default(),
            fallback: None,
        }
    }

    /// Create an [`EmbeddedAssetReader`] loaded with all the assets found by the build script.
    #[must_use]
    pub(crate) fn preloaded() -> Self {
        let mut new = Self {
            loaded: HashMap::default(),
            fallback: None,
        };
        include_all_assets(&mut new);
        new
    }

    /// Create an [`EmbeddedAssetReader`] loaded with all the assets found by the build script.
    #[must_use]
    pub(crate) fn preloaded_with_default(
        mut default: impl FnMut() -> Box<dyn AssetReader> + Send + Sync + 'static,
    ) -> Self {
        let mut new = Self {
            loaded: HashMap::default(),
            fallback: Some(default()),
        };
        include_all_assets(&mut new);
        new
    }

    /// Add an asset to this [`EmbeddedAssetReader`].
    pub(crate) fn add_asset(&mut self, path: &'static Path, data: &'static [u8]) {
        self.loaded.insert(path, data);
    }

    /// Get the data from the asset matching the path provided.
    ///
    /// # Errors
    ///
    /// This will returns an error if the path is not known.
    fn load_path_sync(&self, path: &Path) -> Result<DataReader, AssetReaderError> {
        self.loaded
            .get(path)
            .map(|b| DataReader(b))
            .ok_or_else(|| AssetReaderError::NotFound(path.to_path_buf()))
    }

    fn has_file_sync(&self, path: &Path) -> bool {
        self.loaded.contains_key(path)
    }

    fn is_directory_sync(&self, path: &Path) -> bool {
        let as_folder = path.join("");
        self.loaded
            .keys()
            .any(|loaded_path| loaded_path.starts_with(&as_folder) && loaded_path != &path)
    }

    fn read_directory_sync(&self, path: &Path) -> Result<DirReader, AssetReaderError> {
        if self.is_directory_sync(path) {
            let paths: Vec<_> = self
                .loaded
                .keys()
                .filter(|loaded_path| loaded_path.starts_with(path))
                .map(|t| t.to_path_buf())
                .collect();
            Ok(DirReader(paths))
        } else {
            Err(AssetReaderError::NotFound(path.to_path_buf()))
        }
    }
}

struct DataReader(&'static [u8]);

impl AsyncRead for DataReader {
    fn poll_read(
        self: Pin<&mut Self>,
        _: &mut std::task::Context<'_>,
        buf: &mut [u8],
    ) -> Poll<futures_io::Result<usize>> {
        let read = self.get_mut().0.read(buf);
        Poll::Ready(read)
    }
}

struct DirReader(Vec<PathBuf>);

impl Stream for DirReader {
    type Item = PathBuf;

    fn poll_next(
        self: Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        Poll::Ready(this.0.pop())
    }
}

pub(crate) fn get_meta_path(path: &Path) -> PathBuf {
    let mut meta_path = path.to_path_buf();
    let mut extension = path
        .extension()
        .expect("asset paths must have extensions")
        .to_os_string();
    extension.push(".meta");
    meta_path.set_extension(extension);
    meta_path
}

impl AssetReader for EmbeddedAssetReader {
    fn read<'a>(
        &'a self,
        path: &'a Path,
    ) -> bevy::utils::BoxedFuture<'a, Result<Box<Reader<'a>>, AssetReaderError>> {
        if self.has_file_sync(path) {
            Box::pin(async move {
                self.load_path_sync(path).map(|reader| {
                    let boxed: Box<Reader> = Box::new(reader);
                    boxed
                })
            })
        } else if let Some(fallback) = self.fallback.as_ref() {
            fallback.read(path)
        } else {
            Box::pin(async move { Err(AssetReaderError::NotFound(path.to_path_buf())) })
        }
    }

    fn read_meta<'a>(
        &'a self,
        path: &'a Path,
    ) -> bevy::utils::BoxedFuture<'a, Result<Box<Reader<'a>>, AssetReaderError>> {
        let meta_path = get_meta_path(path);
        if self.has_file_sync(&meta_path) {
            Box::pin(async move {
                self.load_path_sync(&meta_path).map(|reader| {
                    let boxed: Box<Reader> = Box::new(reader);
                    boxed
                })
            })
        } else if let Some(fallback) = self.fallback.as_ref() {
            fallback.read_meta(path)
        } else {
            Box::pin(async move { Err(AssetReaderError::NotFound(meta_path)) })
        }
    }

    fn read_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> bevy::utils::BoxedFuture<'a, Result<Box<PathStream>, AssetReaderError>> {
        Box::pin(async move {
            self.read_directory_sync(path).map(|read_dir| {
                let boxed: Box<PathStream> = Box::new(read_dir);
                boxed
            })
        })
    }

    fn is_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> bevy::utils::BoxedFuture<'a, Result<bool, AssetReaderError>> {
        Box::pin(async move { Ok(self.is_directory_sync(path)) })
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::asset_reader::EmbeddedAssetReader;

    #[cfg_attr(not(target_arch = "wasm32"), test)]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn load_path() {
        let mut embedded = EmbeddedAssetReader::new();
        embedded.add_asset(Path::new("asset.png"), &[1, 2, 3]);
        embedded.add_asset(Path::new("other_asset.png"), &[4, 5, 6]);
        assert!(embedded.load_path_sync(&Path::new("asset.png")).is_ok());
        assert_eq!(
            embedded.load_path_sync(&Path::new("asset.png")).unwrap().0,
            [1, 2, 3]
        );
        assert_eq!(
            embedded
                .load_path_sync(&Path::new("other_asset.png"))
                .unwrap()
                .0,
            [4, 5, 6]
        );
        assert!(embedded.load_path_sync(&Path::new("asset")).is_err());
        assert!(embedded.load_path_sync(&Path::new("other")).is_err());
    }

    #[cfg_attr(not(target_arch = "wasm32"), test)]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn is_directory() {
        let mut embedded = EmbeddedAssetReader::new();
        embedded.add_asset(Path::new("asset.png"), &[]);
        embedded.add_asset(Path::new("directory/asset.png"), &[]);
        assert!(!embedded.is_directory_sync(&Path::new("asset.png")));
        assert!(!embedded.is_directory_sync(&Path::new("asset")));
        assert!(embedded.is_directory_sync(&Path::new("directory")));
        assert!(embedded.is_directory_sync(&Path::new("directory/")));
        assert!(!embedded.is_directory_sync(&Path::new("directory/asset")));
    }

    #[cfg_attr(not(target_arch = "wasm32"), test)]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn read_directory() {
        let mut embedded = EmbeddedAssetReader::new();
        embedded.add_asset(Path::new("asset.png"), &[]);
        embedded.add_asset(Path::new("directory/asset.png"), &[]);
        embedded.add_asset(Path::new("directory/asset2.png"), &[]);
        assert!(embedded
            .read_directory_sync(&Path::new("asset.png"))
            .is_err());
        assert!(embedded
            .read_directory_sync(&Path::new("directory"))
            .is_ok());
        let mut list = embedded
            .read_directory_sync(&Path::new("directory"))
            .unwrap()
            .0
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect::<Vec<_>>();
        list.sort();
        assert_eq!(list, vec!["directory/asset.png", "directory/asset2.png"]);
    }

    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::wasm_bindgen_test;

    #[cfg_attr(not(target_arch = "wasm32"), test)]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn check_preloaded_simple() {
        let embedded = EmbeddedAssetReader::preloaded();

        let path = "example_asset.test";

        let loaded = embedded.load_path_sync(&Path::new(path));
        assert!(loaded.is_ok());
        let raw_asset = loaded.unwrap();
        assert!(String::from_utf8(raw_asset.0.to_vec()).is_ok());
        assert_eq!(String::from_utf8(raw_asset.0.to_vec()).unwrap(), "hello");
    }

    #[cfg_attr(not(target_arch = "wasm32"), test)]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn check_preloaded_special_chars() {
        let embedded = EmbeddedAssetReader::preloaded();

        let path = "açèt.test";

        let loaded = embedded.load_path_sync(&Path::new(path));
        assert!(loaded.is_ok());
        let raw_asset = loaded.unwrap();
        assert!(String::from_utf8(raw_asset.0.to_vec()).is_ok());
        assert_eq!(
            String::from_utf8(raw_asset.0.to_vec()).unwrap(),
            "with special chars"
        );
    }

    #[cfg_attr(not(target_arch = "wasm32"), test)]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn check_preloaded_subdir() {
        let embedded = EmbeddedAssetReader::preloaded();

        let path = "subdir/other_asset.test";

        let loaded = embedded.load_path_sync(&Path::new(path));
        assert!(loaded.is_ok());
        let raw_asset = loaded.unwrap();
        assert!(String::from_utf8(raw_asset.0.to_vec()).is_ok());
        assert_eq!(
            String::from_utf8(raw_asset.0.to_vec()).unwrap(),
            "in subdirectory"
        );
    }
}
