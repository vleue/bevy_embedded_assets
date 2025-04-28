#![cfg(feature = "default-source")]

use std::fmt::Display;

use bevy::{
    asset::{LoadContext, io::Reader},
    prelude::*,
};
use bevy_embedded_assets::{EmbeddedAssetPlugin, PluginMode};
use thiserror::Error;

#[derive(Asset, TypePath, Debug)]
pub struct TestAsset {
    pub value: String,
}

#[derive(Default)]
pub struct TestAssetLoader;

#[derive(Debug, Error)]
pub struct TestError;

impl Display for TestError {
    fn fmt(&self, _: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Ok(())
    }
}

impl bevy::asset::AssetLoader for TestAssetLoader {
    type Asset = TestAsset;
    type Settings = ();
    type Error = TestError;
    async fn load(
        &self,
        reader: &mut dyn Reader,
        _: &(),
        _: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        bevy::asset::AsyncReadExt::read_to_end(reader, &mut bytes)
            .await
            .unwrap();

        Ok(TestAsset {
            value: String::from_utf8(bytes).unwrap(),
        })
    }

    fn extensions(&self) -> &[&str] {
        &["test"]
    }
}

#[test]
fn work_with_embedded_source_plugin_before() {
    let mut app = App::new();
    app.add_plugins(EmbeddedAssetPlugin {
        mode: PluginMode::ReplaceAndFallback {
            path: "runtime_assets".to_string(),
        },
    })
    .add_plugins(DefaultPlugins.set(AssetPlugin {
        file_path: "runtime_assets".to_string(),
        ..default()
    }))
    .init_asset::<TestAsset>()
    .init_asset_loader::<TestAssetLoader>();
    app.finish();

    let asset_server = app.world_mut().resource_mut::<AssetServer>();
    let handle_1: Handle<TestAsset> = asset_server.load("example_asset.test");
    let handle_2: Handle<TestAsset> = asset_server.load("açèt.test");
    let handle_3: Handle<TestAsset> = asset_server.load("subdir/other_asset.test");
    let handle_4: Handle<TestAsset> = asset_server.load("asset.test");
    app.update();
    let test_assets = app.world_mut().resource_mut::<Assets<TestAsset>>();
    let asset = test_assets.get(&handle_1).unwrap();
    assert_eq!(asset.value, "hello");
    let asset = test_assets.get(&handle_2).unwrap();
    assert_eq!(asset.value, "with special chars");
    let asset = test_assets.get(&handle_3).unwrap();
    assert_eq!(asset.value, "in subdirectory");
    let asset = test_assets.get(&handle_4).unwrap();
    assert_eq!(asset.value, "at runtime");
}
