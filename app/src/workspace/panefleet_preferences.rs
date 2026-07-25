use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub(crate) const PANEFLEET_WORKSPACE_PREFERENCES_VERSION: u32 = 1;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub(crate) struct PaneFleetWorkspacePreferences {
    #[serde(default = "current_version")]
    pub version: u32,
    #[serde(default = "default_true")]
    pub show_workspace_path: bool,
    #[serde(default = "default_true")]
    pub show_git_branch: bool,
    #[serde(default = "default_true")]
    pub show_agent_activity: bool,
}

fn current_version() -> u32 {
    PANEFLEET_WORKSPACE_PREFERENCES_VERSION
}

fn default_true() -> bool {
    true
}

impl Default for PaneFleetWorkspacePreferences {
    fn default() -> Self {
        Self {
            version: PANEFLEET_WORKSPACE_PREFERENCES_VERSION,
            show_workspace_path: true,
            show_git_branch: true,
            show_agent_activity: true,
        }
    }
}

impl PaneFleetWorkspacePreferences {
    pub fn path() -> PathBuf {
        warp_core::paths::state_dir().join("panefleet-workspace-preferences.json")
    }

    pub fn decode(contents: &[u8]) -> serde_json::Result<Self> {
        serde_json::from_slice(contents)
    }

    pub fn load_or_default(path: &Path) -> Self {
        fs::read(path)
            .ok()
            .and_then(|contents| Self::decode(&contents).ok())
            .filter(|preferences| preferences.version <= PANEFLEET_WORKSPACE_PREFERENCES_VERSION)
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PaneFleetWorkspaceIcon {
    Github,
    Git,
    PaneFleet,
}

pub(crate) fn workspace_icon_for_path(workspace_path: &Path) -> PaneFleetWorkspaceIcon {
    let Some(git_dir) = resolve_git_dir(workspace_path) else {
        return PaneFleetWorkspaceIcon::PaneFleet;
    };

    let mut config_paths = vec![git_dir.join("config")];
    if let Ok(common_dir) = fs::read_to_string(git_dir.join("commondir")) {
        let common_dir = PathBuf::from(common_dir.trim());
        let common_dir = if common_dir.is_absolute() {
            common_dir
        } else {
            git_dir.join(common_dir)
        };
        config_paths.push(common_dir.join("config"));
    }

    if config_paths.into_iter().any(|path| {
        fs::read_to_string(path).is_ok_and(|config| git_config_references_github(&config))
    }) {
        PaneFleetWorkspaceIcon::Github
    } else {
        PaneFleetWorkspaceIcon::Git
    }
}

pub(crate) fn git_branch_for_workspace(workspace_path: &Path) -> Option<String> {
    let git_dir = resolve_git_dir(workspace_path)?;
    let head = fs::read_to_string(git_dir.join("HEAD")).ok()?;
    parse_git_head(&head)
}

fn resolve_git_dir(workspace_path: &Path) -> Option<PathBuf> {
    let dot_git = workspace_path.join(".git");
    if dot_git.is_dir() {
        Some(dot_git)
    } else {
        let contents = fs::read_to_string(dot_git).ok()?;
        let git_dir = contents.trim().strip_prefix("gitdir:")?.trim();
        let git_dir = PathBuf::from(git_dir);
        if git_dir.is_absolute() {
            Some(git_dir)
        } else {
            Some(workspace_path.join(git_dir))
        }
    }
}

fn git_config_references_github(config: &str) -> bool {
    config
        .lines()
        .filter_map(|line| line.split_once('='))
        .any(|(key, value)| {
            key.trim().eq_ignore_ascii_case("url")
                && value.trim().to_ascii_lowercase().contains("github.com")
        })
}

fn parse_git_head(head: &str) -> Option<String> {
    let head = head.trim();
    if let Some(reference) = head.strip_prefix("ref:") {
        return reference
            .trim()
            .strip_prefix("refs/heads/")
            .map(str::to_owned);
    }
    (!head.is_empty()).then(|| head.chars().take(8).collect())
}

#[cfg(test)]
#[path = "panefleet_preferences_tests.rs"]
mod tests;
