# Bevy Embedded Assets

![MIT/Apache 2.0](https://img.shields.io/badge/license-MIT%2FApache-blue.svg)
[![Realease Doc](https://docs.rs/bevy_embedded_assets/badge.svg)](https://docs.rs/bevy_embedded_assets)
[![Crate](https://img.shields.io/crates/v/bevy_embedded_assets.svg)](https://crates.io/crates/bevy_embedded_assets)
[![Bevy Tracking](https://img.shields.io/badge/Bevy%20tracking-main-lightblue)](https://github.com/bevyengine/bevy/blob/main/docs/plugins_guidelines.md#main-branch-tracking)
[![CI](https://github.com/vleue/embedded_assets/actions/workflows/ci.yaml/badge.svg)](https://github.com/vleue/embedded_assets/actions/workflows/ci.yaml)

Embed your asset folder inside your binary for easier releases.

```rust
use bevy::prelude::*;
use bevy_embedded_assets::EmbeddedAssetPlugin;

fn main() {
    App::new().add_plugins((EmbeddedAssetPlugin::default(), DefaultPlugins));
}
```

`EmbeddedAssetPlugin` has three modes:

* `PluginMode::AutoLoad` will embed the asset folder and make it available through the `embedded://` source
* `PluginMode::ReplaceDefault` will embed the asset folder and make it available through the default source
* `PluginMode::ReplaceAndFallback` will embed the asset folder and make it available through the default source. If a fail is not found at runtime, it fallback to the default source for the current platform

## Bevy Compatibility

|Bevy|bevy_embedded_assets|
|---|---|
|main|main|
|0.16|0.13|
|0.15|0.12|
|0.14|0.11|
|0.13|0.10|
|0.12|0.9|
|0.11|0.8|
|0.10|0.7|
|0.9|0.6|
|0.8|0.4|
|0.7|0.3|
|0.6|0.2|
|0.5|0.1|
