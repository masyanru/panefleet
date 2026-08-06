use anyhow::Result;
use objc2_foundation::NSBundle;

/// Apple Developer Team ID used for code signing and validation.
pub const APPLE_TEAM_ID: &str = "2BBY89MBSN";

/// PaneFleet's Apple Developer Team ID.
///
/// Keep this separate from [`APPLE_TEAM_ID`]: the latter remains part of inherited Warp
/// distribution and app-group behavior.
pub const PANEFLEET_APPLE_TEAM_ID: &str = "7HHQ872HRQ";

/// Get the path to the macOS `.app` bundle.
pub fn get_bundle_path() -> Result<String> {
    let bundle = NSBundle::mainBundle();
    let path = bundle.bundlePath();
    Ok(path.to_string())
}
