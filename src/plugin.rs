use bevy::prelude::*;
use bevy::tasks::IoTaskPool;

/// Bevy plugin to add to your application that will insert a custom [`AssetServer`] embedding
/// your assets instead of the default added by the [`AssetPlugin`](bevy::asset::AssetPlugin).
/// If you are using the [`DefaultPlugins`] group from Bevy, it can be added this way:
///
/// ```rust
/// # use bevy::prelude::*;
/// # use bevy_embedded_assets::EmbeddedAssetPlugin;
/// # fn main() {
///     App::build().add_plugins_with(DefaultPlugins, |group| {
///         group.add_before::<bevy::asset::AssetPlugin, _>(EmbeddedAssetPlugin)
///     });
/// # }
/// ```
#[allow(missing_debug_implementations, missing_copy_implementations)]
#[derive(Default)]
pub struct EmbeddedAssetPlugin;

impl Plugin for EmbeddedAssetPlugin {
    fn build(&self, app: &mut AppBuilder) {
        let task_pool = app
            .world()
            .get_resource::<IoTaskPool>()
            .expect("`IoTaskPool` resource not found.")
            .0
            .clone();

        app.insert_resource(AssetServer::new(
            crate::EmbeddedAssetIo::preloaded(),
            task_pool,
        ));
    }
}
