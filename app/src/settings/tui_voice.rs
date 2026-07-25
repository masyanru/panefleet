use serde::{Deserialize, Serialize};
use settings::macros::define_settings_group;
use settings::{SupportedPlatforms, SyncToCloud};

use super::ai::VoiceInputToggleKey;

/// The voice-input key setting used by the headless TUI.
///
/// This transparent wrapper reuses the GUI enum's TOML value space while
/// allowing the settings system to register a second, TUI-only setting group.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(transparent)]
#[schemars(transparent)]
pub struct TuiVoiceInputToggleKey(pub VoiceInputToggleKey);

impl settings_value::SettingsValue for TuiVoiceInputToggleKey {}

impl TuiVoiceInputToggleKey {
    pub fn keystroke(self) -> Option<warpui::keymap::Keystroke> {
        self.0.keystroke()
    }
}

impl From<VoiceInputToggleKey> for TuiVoiceInputToggleKey {
    fn from(value: VoiceInputToggleKey) -> Self {
        Self(value)
    }
}

impl From<TuiVoiceInputToggleKey> for VoiceInputToggleKey {
    fn from(value: TuiVoiceInputToggleKey) -> Self {
        value.0
    }
}

define_settings_group!(TuiVoiceSettings, settings: [
    voice_input_toggle_key: TuiVoiceInputToggleKeySetting {
        type: TuiVoiceInputToggleKey,
        default: TuiVoiceInputToggleKey::default(),
        supported_platforms: SupportedPlatforms::DESKTOP,
        sync_to_cloud: SyncToCloud::Never,
        surface: settings::SettingSurfaces::TUI,
        private: false,
        toml_path: "agents.voice.voice_input_toggle_key",
        description: "An additional key that starts voice input in the Warp Agent CLI. The hardcoded ctrl-s binding remains; tap to start and press Escape or Enter to stop. Defaults to none. Fn is unsupported and Super may be unavailable in some terminals.",
    },
]);

#[cfg(test)]
#[path = "tui_voice_tests.rs"]
mod tests;
