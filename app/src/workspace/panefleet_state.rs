use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::terminal::CLIAgent;

pub const PANEFLEET_STATE_VERSION: u32 = 2;
const CODEX_STANDALONE_COMMAND: &str = "env -u CODEX_THREAD_ID -u CODEX_CI codex";

fn legacy_state_version() -> u32 {
    1
}

#[derive(Debug, Deserialize, Serialize)]
pub(super) struct PaneFleetPersistedState {
    #[serde(default = "legacy_state_version")]
    pub version: u32,
    pub active_project: Option<PathBuf>,
    #[serde(default)]
    pub workspaces: Vec<PaneFleetPersistedWorkspace>,
}

impl PaneFleetPersistedState {
    pub fn new(
        active_project: Option<PathBuf>,
        workspaces: Vec<PaneFleetPersistedWorkspace>,
    ) -> Self {
        Self {
            version: PANEFLEET_STATE_VERSION,
            active_project,
            workspaces,
        }
    }

    pub fn decode(contents: &[u8]) -> serde_json::Result<Self> {
        let mut state = serde_json::from_slice::<Self>(contents)?;
        if state.version < PANEFLEET_STATE_VERSION {
            state.version = PANEFLEET_STATE_VERSION;
        }
        Ok(state)
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub(super) struct PaneFleetPersistedWorkspace {
    pub path: PathBuf,
    #[serde(default)]
    pub active_tab_index: usize,
    #[serde(default)]
    pub tabs: Vec<PaneFleetPersistedTab>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(super) struct PaneFleetPersistedTab {
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_session: Option<PaneFleetPersistedAgentSession>,
}

impl PaneFleetPersistedTab {
    pub fn terminal(title: Option<String>) -> Self {
        Self {
            title,
            agent_session: None,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub(super) struct PaneFleetPersistedAgentSession {
    pub agent: CLIAgent,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_session_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum PaneFleetResumeError {
    MissingSessionId,
    InvalidSessionId,
    UnsupportedAgent(CLIAgent),
}

impl PaneFleetResumeError {
    pub fn message(&self) -> String {
        match self {
            Self::MissingSessionId => "The agent did not report a resumable session ID".to_string(),
            Self::InvalidSessionId => "The saved agent session ID is invalid".to_string(),
            Self::UnsupportedAgent(agent) => {
                format!(
                    "{} does not support automatic resume yet",
                    agent.display_name()
                )
            }
        }
    }
}

impl PaneFleetPersistedAgentSession {
    pub fn new(agent: CLIAgent, provider_session_id: Option<String>) -> Self {
        Self {
            agent,
            provider_session_id,
        }
    }

    pub fn resume_command(&self) -> Result<String, PaneFleetResumeError> {
        let session_id = self
            .provider_session_id
            .as_deref()
            .map(str::trim)
            .filter(|session_id| !session_id.is_empty())
            .ok_or(PaneFleetResumeError::MissingSessionId)?;

        match self.agent {
            CLIAgent::Claude => {
                let session_id = Uuid::parse_str(session_id)
                    .map_err(|_| PaneFleetResumeError::InvalidSessionId)?;
                Ok(format!("claude --resume {session_id}"))
            }
            CLIAgent::Codex => {
                let session_id = Uuid::parse_str(session_id)
                    .map_err(|_| PaneFleetResumeError::InvalidSessionId)?;
                Ok(format!("{CODEX_STANDALONE_COMMAND} resume {session_id}"))
            }
            CLIAgent::OpenCode => Ok(format!("opencode -s {}", shell_words::quote(session_id))),
            CLIAgent::Gemini
            | CLIAgent::Amp
            | CLIAgent::Droid
            | CLIAgent::Copilot
            | CLIAgent::Pi
            | CLIAgent::OhMyPi
            | CLIAgent::Auggie
            | CLIAgent::CursorCli
            | CLIAgent::Goose
            | CLIAgent::Hermes
            | CLIAgent::Vibe
            | CLIAgent::Antigravity
            | CLIAgent::Unknown => Err(PaneFleetResumeError::UnsupportedAgent(self.agent)),
        }
    }
}

pub(super) fn panefleet_agent_launch_command(agent: CLIAgent) -> String {
    match agent {
        // PaneFleet itself is commonly launched from Codex CLI while developing. A child shell
        // inherits CODEX_THREAD_ID and CODEX_CI from that parent unless we remove them, causing a
        // new `codex` process to attach to the parent's conversation instead of creating a
        // workspace-owned session.
        CLIAgent::Codex => CODEX_STANDALONE_COMMAND.to_string(),
        _ => agent.command_prefix().to_string(),
    }
}

pub(super) fn write_panefleet_state_atomic(
    path: &Path,
    state: &PaneFleetPersistedState,
) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let temporary_path = path.with_extension("json.tmp");
    let contents = serde_json::to_vec_pretty(state).map_err(io::Error::other)?;
    fs::write(&temporary_path, contents)?;
    match fs::rename(&temporary_path, path) {
        Ok(()) => Ok(()),
        Err(error) => {
            let _ = fs::remove_file(&temporary_path);
            Err(error)
        }
    }
}

#[cfg(test)]
#[path = "panefleet_state_tests.rs"]
mod tests;
