use std::path::Path;

use bevy_embedded_assets::EmbeddedAssetIo;

#[test]
fn check_preloaded() {
    let embedded = EmbeddedAssetIo::preloaded();

    assert!(embedded.load_path_sync(&Path::new("example_asset")).is_ok());
    let raw_asset = embedded
        .load_path_sync(&Path::new("example_asset"))
        .unwrap();
    assert!(String::from_utf8(raw_asset.clone()).is_ok());
    assert_eq!(String::from_utf8(raw_asset).unwrap(), "hello");

    assert!(embedded.load_path_sync(&Path::new("açèt")).is_ok());
    let raw_asset = embedded.load_path_sync(&Path::new("açèt")).unwrap();
    assert!(String::from_utf8(raw_asset.clone()).is_ok());
    assert_eq!(String::from_utf8(raw_asset).unwrap(), "with special chars");
}
