use settings::schema::SettingSchemaEntry;
use settings::{Setting, SettingSurfaces, SettingsMode, SyncToCloud};
use settings_value::SettingsValue;

use super::{TuiVoiceInputToggleKey, TuiVoiceInputToggleKeySetting};
use crate::settings::VoiceInputToggleKey;

#[test]
fn tui_voice_setting_defaults_to_none_and_round_trips() {
    assert_eq!(
        TuiVoiceInputToggleKeySetting::default_value(),
        TuiVoiceInputToggleKey::default()
    );
    assert_eq!(
        TuiVoiceInputToggleKey::default().to_file_value(),
        serde_json::json!("none")
    );

    let value = serde_json::json!("control_left");
    let parsed = TuiVoiceInputToggleKey::from_file_value(&value).expect("valid enum value");
    assert_eq!(parsed.0, VoiceInputToggleKey::ControlLeft);
    assert_eq!(parsed.to_file_value(), value);
}

#[test]
fn tui_voice_setting_is_local_and_tui_only() {
    assert_eq!(
        TuiVoiceInputToggleKeySetting::toml_path(),
        Some("agents.voice.voice_input_toggle_key")
    );
    assert_eq!(
        TuiVoiceInputToggleKeySetting::sync_to_cloud(),
        SyncToCloud::Never
    );

    let entry = inventory::iter::<SettingSchemaEntry>
        .into_iter()
        .find(|entry| {
            entry.storage_key == "voice_input_toggle_key" && {
                let surfaces = (entry.surfaces_fn)();
                surfaces.includes(SettingsMode::Tui) && !surfaces.includes(SettingsMode::Gui)
            }
        })
        .expect("TUI voice setting schema entry");
    assert_eq!(entry.hierarchy, Some("agents.voice"));
    assert_eq!((entry.surfaces_fn)(), SettingSurfaces::TUI);
    assert!(entry.description.contains("Fn"));
    assert!(entry.description.contains("Super"));
}
