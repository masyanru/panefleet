use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub(crate) const PANEFLEET_NOTIFICATION_PREFERENCES_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PaneFleetNotificationSound {
    #[default]
    Glass,
    Pop,
    Tink,
}

impl PaneFleetNotificationSound {
    pub const ALL: [Self; 3] = [Self::Glass, Self::Pop, Self::Tink];

    pub fn display_name(self) -> &'static str {
        match self {
            Self::Glass => "Glass",
            Self::Pop => "Pop",
            Self::Tink => "Tink",
        }
    }

    fn system_name(self) -> &'static str {
        self.display_name()
    }

    pub fn play(self) {
        play_system_sound(self.system_name(), 0.45);
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub(crate) struct PaneFleetNotificationPreferences {
    #[serde(default = "current_version")]
    pub version: u32,
    #[serde(default = "default_true")]
    pub agent_completion_sound_enabled: bool,
    #[serde(default)]
    pub agent_completion_sound: PaneFleetNotificationSound,
}

fn current_version() -> u32 {
    PANEFLEET_NOTIFICATION_PREFERENCES_VERSION
}

fn default_true() -> bool {
    true
}

impl Default for PaneFleetNotificationPreferences {
    fn default() -> Self {
        Self {
            version: PANEFLEET_NOTIFICATION_PREFERENCES_VERSION,
            agent_completion_sound_enabled: true,
            agent_completion_sound: PaneFleetNotificationSound::Glass,
        }
    }
}

impl PaneFleetNotificationPreferences {
    pub fn path() -> PathBuf {
        warp_core::paths::state_dir().join("panefleet-notification-preferences.json")
    }

    pub fn decode(contents: &[u8]) -> serde_json::Result<Self> {
        serde_json::from_slice(contents)
    }

    pub fn load_or_default(path: &Path) -> Self {
        fs::read(path)
            .ok()
            .and_then(|contents| Self::decode(&contents).ok())
            .filter(|preferences| preferences.version <= PANEFLEET_NOTIFICATION_PREFERENCES_VERSION)
            .unwrap_or_default()
    }

    pub fn write_atomic(&self, path: &Path) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let temporary_path = path.with_extension("json.tmp");
        let contents = serde_json::to_vec_pretty(self).map_err(io::Error::other)?;
        fs::write(&temporary_path, contents)?;
        match fs::rename(&temporary_path, path) {
            Ok(()) => Ok(()),
            Err(error) => {
                let _ = fs::remove_file(&temporary_path);
                Err(error)
            }
        }
    }

    pub fn play_agent_completion_sound(&self) {
        if self.agent_completion_sound_enabled {
            self.agent_completion_sound.play();
        }
    }
}

#[cfg(target_os = "macos")]
fn play_system_sound(sound_name: &str, volume: f32) {
    use std::ffi::CString;
    use std::os::raw::c_char;

    unsafe extern "C" {
        fn panefleet_play_system_sound(sound_name: *const c_char, volume: f32);
    }

    let Ok(sound_name) = CString::new(sound_name) else {
        return;
    };
    unsafe {
        panefleet_play_system_sound(sound_name.as_ptr(), volume);
    }
}

#[cfg(not(target_os = "macos"))]
fn play_system_sound(_sound_name: &str, _volume: f32) {}

#[cfg(test)]
#[path = "panefleet_notifications_tests.rs"]
mod tests;
