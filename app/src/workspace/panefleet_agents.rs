use std::fs;
use std::io;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::terminal::CLIAgent;

pub(crate) const PANEFLEET_AGENT_DEFINITIONS_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PaneFleetPromptTransport {
    Argv,
    Stdin,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub(crate) struct PaneFleetAgentDefinition {
    pub id: String,
    pub label: String,
    pub agent: CLIAgent,
    pub executable: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub prompt_only_args: Vec<String>,
    pub prompt_transport: PaneFleetPromptTransport,
    /// Whether two of these may run at once anywhere on this machine.
    ///
    /// Codex keeps its subscription tokens in a shared `~/.codex/auth.json`
    /// and rewrites them on refresh, so a second process invalidates the
    /// first. Claude and OpenCode have no such constraint.
    #[serde(default)]
    pub single_instance_per_machine: bool,
    #[serde(default = "default_true")]
    pub enabled_in_launcher: bool,
    #[serde(default)]
    pub launcher_order: u32,
    #[serde(default)]
    pub bundled: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub(crate) struct PaneFleetAgentDefinitions {
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
        let tokens = self.command_tokens(additional_args, false)?;
        Some(join_tokens(&tokens))
    }

    /// Builds the launch command for an agent that should start working on
    /// something immediately, delivering the prompt the way this definition
    /// declares.
    ///
    /// A blank prompt falls back to a plain launch rather than an empty one:
    /// `prompt_only_args` typically carries the flag that puts the agent in
    /// one-shot mode, and passing it with nothing to work on leaves the agent
    /// waiting on input that never comes.
    pub fn launch_command_with_prompt<'a>(
        &self,
        prompt: &str,
        additional_args: impl IntoIterator<Item = &'a str>,
    ) -> Option<String> {
        let prompt = prompt.trim();
        if prompt.is_empty() {
            return self.launch_command(additional_args);
        }

        let mut tokens = self.command_tokens(additional_args, true)?;
        match self.prompt_transport {
            PaneFleetPromptTransport::Argv => {
                tokens.push(prompt.to_string());
                Some(join_tokens(&tokens))
            }
            // `printf` rather than `echo` so a prompt starting with `-`, or
            // containing backslashes, arrives byte for byte.
            PaneFleetPromptTransport::Stdin => Some(format!(
                "printf '%s' {} | {}",
                shell_words::quote(prompt),
                join_tokens(&tokens)
            )),
        }
    }

    fn command_tokens<'a>(
        &self,
        additional_args: impl IntoIterator<Item = &'a str>,
        with_prompt: bool,
    ) -> Option<Vec<String>> {
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
        if with_prompt {
            tokens.extend(self.prompt_only_args.iter().cloned());
        }
        tokens.extend(additional_args.into_iter().map(str::to_string));
        Some(tokens)
    }
}

/// Explains why a second copy of `definition` must not start, given the agents
/// already running, or `None` when starting one is fine.
///
/// The check only sees agents PaneFleet itself launched. It cannot know about a
/// Codex started from a plain terminal, so the message says what was checked
/// rather than claiming the machine is clear.
pub(crate) fn single_instance_conflict(
    definition: &PaneFleetAgentDefinition,
    running_agents: impl IntoIterator<Item = CLIAgent>,
) -> Option<String> {
    if !definition.single_instance_per_machine {
        return None;
    }
    running_agents
        .into_iter()
        .any(|running| running == definition.agent)
        .then(|| {
            format!(
                "{} is already running in another PaneFleet tab. It keeps its credentials in one \
                 shared file and rewrites them when they refresh, so a second copy would sign the \
                 first one out. Close that tab first.",
                definition.label
            )
        })
}

fn join_tokens(tokens: &[String]) -> String {
    tokens
        .iter()
        .map(|token| shell_words::quote(token).into_owned())
        .collect::<Vec<_>>()
        .join(" ")
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
                    single_instance_per_machine: true,
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
                    single_instance_per_machine: false,
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
                    single_instance_per_machine: false,
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

    pub fn bundled_default(id: &str) -> Option<PaneFleetAgentDefinition> {
        Self::bundled_defaults()
            .definitions
            .into_iter()
            .find(|definition| definition.id == id)
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
