#![allow(clippy::needless_doctest_main)]
#![doc = include_str!("../README.md")]
#![warn(
    missing_debug_implementations,
    missing_copy_implementations,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unstable_features,
    unused_import_braces,
    unused_qualifications,
    missing_docs,
    clippy::pedantic
)]

use std::path::PathBuf;

use bevy_app::App;
use bevy_app::Plugin;
use bevy_asset::AssetPlugin;
use bevy_asset::io::embedded::EmbeddedAssetRegistry;
use bevy_ecs::resource::Resource;
#[cfg(feature = "default-source")]
use {
    bevy_asset::{
        AssetApp,
        io::{AssetSource, AssetSourceId},
    },
    log::error,
};

#[cfg(feature = "default-source")]
mod asset_reader;
#[cfg(feature = "default-source")]
pub use {asset_reader::DataReader, asset_reader::EmbeddedAssetReader};

include!(concat!(env!("OUT_DIR"), "/include_all_assets.rs"));

/// Bevy plugin to embed all your asset folder.
///
/// If using the default value of the plugin, or using [`PluginMode::AutoLoad`], assets will be
/// available using the `embedded://` asset source.
///
/// Order of plugins is not important in this mode, it can be added before or after the
/// `AssetPlugin`.
///
/// ```rust
/// # use bevy::prelude::*;
/// # use bevy_embedded_assets::EmbeddedAssetPlugin;
/// # #[derive(Asset, TypePath)]
/// # pub struct MyAsset;
/// # fn main() {
/// # let mut app = App::new();
/// app.add_plugins((EmbeddedAssetPlugin::default(), DefaultPlugins));
/// # app.init_asset::<MyAsset>();
/// # let asset_server: Mut<'_, AssetServer> = app.world_mut().resource_mut::<AssetServer>();
/// let handle: Handle<MyAsset> = asset_server.load("embedded://example_asset.test");
/// # }
/// ```
///
/// If using [`PluginMode::ReplaceDefault`] or  [`PluginMode::ReplaceAndFallback`], assets will be
/// available using the default asset source.
///
/// Order of plugins is important in these modes, it must be added before the `AssetPlugin`.
///
/// ```rust
/// # use bevy::prelude::*;
/// # use bevy_embedded_assets::{EmbeddedAssetPlugin, PluginMode};
/// # #[derive(Asset, TypePath)]
/// # pub struct MyAsset;
/// # fn main() {
/// # let mut app = App::new();
/// app.add_plugins((EmbeddedAssetPlugin { mode: PluginMode::ReplaceDefault }, DefaultPlugins));
/// # app.init_asset::<MyAsset>();
/// # let asset_server: Mut<'_, AssetServer> = app.world_mut().resource_mut::<AssetServer>();
/// let handle: Handle<MyAsset> = asset_server.load("example_asset.test");
/// # }
/// ```
///
///
#[allow(clippy::module_name_repetitions)]
#[derive(Default, Debug, Clone)]
pub struct EmbeddedAssetPlugin {
    /// How this plugin should behave.
    pub mode: PluginMode,
}

/// How [`EmbeddedAssetPlugin`] should behave.
#[derive(Debug, Clone, Default)]
#[allow(missing_copy_implementations)]
pub enum PluginMode {
    /// Embed the assets folder and make the files available through the `embedded://` source.
    #[default]
    AutoLoad,
    /// Replace the default asset source with an embedded source.
    ///
    /// In this mode, listing files in a directory will work in wasm.
    #[cfg(feature = "default-source")]
    ReplaceDefault,
    /// Replace the default asset source with an embedded source. If a file is not present at build
    /// time, fallback to the default source for the current platform.
    ///
    /// In this mode, listing files in a directory will work in wasm.
    #[cfg(feature = "default-source")]
    ReplaceAndFallback {
        /// The default file path to use (relative to the project root). `"assets"` is the
        /// standard value in Bevy.
        path: String,
    },
}

#[derive(Resource, Default)]
struct AllTheEmbedded;

trait EmbeddedRegistry {
    fn insert_included_asset(&mut self, name: &'static str, bytes: &'static [u8]);
}

impl EmbeddedRegistry for &mut EmbeddedAssetRegistry {
    fn insert_included_asset(&mut self, name: &str, bytes: &'static [u8]) {
        self.insert_asset(PathBuf::new(), std::path::Path::new(name), bytes);
    }
}

impl Plugin for EmbeddedAssetPlugin {
    fn build(&self, app: &mut App) {
        match &self.mode {
            PluginMode::AutoLoad => {
                if app.is_plugin_added::<AssetPlugin>() {
                    let mut registry = app.world_mut().resource_mut::<EmbeddedAssetRegistry>();
                    include_all_assets(registry.as_mut());
                    app.init_resource::<AllTheEmbedded>();
                }
            }
            #[cfg(feature = "default-source")]
            PluginMode::ReplaceDefault => {
                if app.is_plugin_added::<AssetPlugin>() {
                    error!(
                        "plugin EmbeddedAssetPlugin must be added before plugin AssetPlugin when replacing the default asset source"
                    );
                }
                app.register_asset_source(
                    AssetSourceId::Default,
                    AssetSource::build()
                        .with_reader(|| Box::new(EmbeddedAssetReader::preloaded()))
                        .with_processed_reader(|| Box::new(EmbeddedAssetReader::preloaded())),
                );
            }
            #[cfg(feature = "default-source")]
            PluginMode::ReplaceAndFallback { path } => {
                if app.is_plugin_added::<AssetPlugin>() {
                    error!(
                        "plugin EmbeddedAssetPlugin must be added before plugin AssetPlugin when replacing the default asset source"
                    );
                }
                let path = path.clone();
                app.register_asset_source(
                    AssetSourceId::Default,
                    AssetSource::build().with_reader(move || {
                        Box::new(EmbeddedAssetReader::preloaded_with_default(
                            AssetSource::get_default_reader(path.clone()),
                        ))
                    }),
                );
            }
        }
    }

    fn finish(&self, app: &mut App) {
        if matches!(self.mode, PluginMode::AutoLoad)
            && app
                .world_mut()
                .remove_resource::<AllTheEmbedded>()
                .is_none()
        {
            let mut registry = app.world_mut().resource_mut::<EmbeddedAssetRegistry>();
            include_all_assets(registry.as_mut());
        }
    }
}
