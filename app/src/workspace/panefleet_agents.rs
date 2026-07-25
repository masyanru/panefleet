use std::fs;
use std::io;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::terminal::CLIAgent;

pub(super) const PANEFLEET_AGENT_DEFINITIONS_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(super) enum PaneFleetPromptTransport {
    Argv,
    Stdin,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub(super) struct PaneFleetAgentDefinition {
    pub id: String,
    pub label: String,
    pub agent: CLIAgent,
    pub executable: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub prompt_only_args: Vec<String>,
    pub prompt_transport: PaneFleetPromptTransport,
    #[serde(default = "default_true")]
    pub enabled_in_launcher: bool,
    #[serde(default)]
    pub launcher_order: u32,
    #[serde(default)]
    pub bundled: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub(super) struct PaneFleetAgentDefinitions {
    pub version: u32,
    #[serde(default)]
    pub definitions: Vec<PaneFleetAgentDefinition>,
}

fn default_true() -> bool {
    true
}

impl PaneFleetAgentDefinition {
    pub fn launch_command<'a>(
        &self,
        additional_args: impl IntoIterator<Item = &'a str>,
    ) -> Option<String> {
        let executable = self.executable.trim();
        if executable.is_empty() {
            return None;
        }

        let mut tokens = Vec::new();
        if self.agent == CLIAgent::Codex {
            tokens.extend([
                "env".to_string(),
                "-u".to_string(),
                "CODEX_THREAD_ID".to_string(),
                "-u".to_string(),
                "CODEX_CI".to_string(),
            ]);
        }
        tokens.push(executable.to_string());
        tokens.extend(self.args.iter().cloned());
        tokens.extend(additional_args.into_iter().map(str::to_string));

        Some(
            tokens
                .iter()
                .map(|token| shell_words::quote(token).into_owned())
                .collect::<Vec<_>>()
                .join(" "),
        )
    }
}

impl PaneFleetAgentDefinitions {
    pub fn bundled_defaults() -> Self {
        Self {
            version: PANEFLEET_AGENT_DEFINITIONS_VERSION,
            definitions: vec![
                PaneFleetAgentDefinition {
                    id: "builtin.codex".to_string(),
                    label: "Codex".to_string(),
                    agent: CLIAgent::Codex,
                    executable: "codex".to_string(),
                    args: Vec::new(),
                    prompt_only_args: Vec::new(),
                    prompt_transport: PaneFleetPromptTransport::Argv,
                    enabled_in_launcher: true,
                    launcher_order: 10,
                    bundled: true,
                },
                PaneFleetAgentDefinition {
                    id: "builtin.claude".to_string(),
                    label: "Claude".to_string(),
                    agent: CLIAgent::Claude,
                    executable: "claude".to_string(),
                    args: Vec::new(),
                    prompt_only_args: Vec::new(),
                    prompt_transport: PaneFleetPromptTransport::Argv,
                    enabled_in_launcher: true,
                    launcher_order: 20,
                    bundled: true,
                },
                PaneFleetAgentDefinition {
                    id: "builtin.opencode".to_string(),
                    label: "OpenCode".to_string(),
                    agent: CLIAgent::OpenCode,
                    executable: "opencode".to_string(),
                    args: Vec::new(),
                    prompt_only_args: Vec::new(),
                    prompt_transport: PaneFleetPromptTransport::Argv,
                    enabled_in_launcher: true,
                    launcher_order: 30,
                    bundled: true,
                },
            ],
        }
    }

    pub fn decode(contents: &[u8]) -> serde_json::Result<Self> {
        serde_json::from_slice(contents)
    }

    pub fn load_or_default(path: &Path) -> Self {
        fs::read(path)
            .ok()
            .and_then(|contents| Self::decode(&contents).ok())
            .filter(|definitions| definitions.version <= PANEFLEET_AGENT_DEFINITIONS_VERSION)
            .unwrap_or_else(Self::bundled_defaults)
    }

    pub fn enabled_launchers(&self) -> Vec<&PaneFleetAgentDefinition> {
        let mut definitions = self
            .definitions
            .iter()
            .filter(|definition| definition.enabled_in_launcher)
            .collect::<Vec<_>>();
        definitions.sort_by_key(|definition| definition.launcher_order);
        definitions
    }

    pub fn first_for_agent(&self, agent: CLIAgent) -> Option<&PaneFleetAgentDefinition> {
        self.enabled_launchers()
            .into_iter()
            .find(|definition| definition.agent == agent)
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

#[cfg(test)]
#[path = "panefleet_agents_tests.rs"]
mod tests;
