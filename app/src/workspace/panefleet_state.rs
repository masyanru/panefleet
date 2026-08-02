use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::terminal::CLIAgent;

pub const PANEFLEET_STATE_VERSION: u32 = 3;
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
    pub source: PaneFleetWorkspaceSource,
    #[serde(default)]
    pub active_tab_index: usize,
    #[serde(default)]
    pub tabs: Vec<PaneFleetPersistedTab>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(super) enum PaneFleetWorkspaceSource {
    #[default]
    ExistingFolder,
    IsolatedWorktree {
        source_repository: PathBuf,
        branch: String,
        #[serde(default)]
        managed: bool,
    },
}

impl PaneFleetWorkspaceSource {
    pub(super) fn project_root(&self, workspace_path: &Path) -> PathBuf {
        match self {
            Self::ExistingFolder => workspace_path.to_path_buf(),
            Self::IsolatedWorktree {
                source_repository, ..
            } => source_repository.clone(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(super) struct PaneFleetPersistedTab {
    pub title: Option<String>,
    /// Written before a tab could hold more than one agent. Read so those files
    /// still restore, never written again.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_session: Option<PaneFleetPersistedAgentSession>,
    /// One entry per pane running an agent. A tab commonly holds two — the same
    /// agent twice, or a builder and a critic — and each has to come back into
    /// its own pane.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub agent_sessions: Vec<PaneFleetPersistedAgentSession>,
}

impl PaneFleetPersistedTab {
    pub fn terminal(title: Option<String>) -> Self {
        Self {
            title,
            agent_session: None,
            agent_sessions: Vec::new(),
        }
    }

    /// Every agent session of this tab, whichever field carried it.
    pub fn sessions(self) -> Vec<PaneFleetPersistedAgentSession> {
        if self.agent_sessions.is_empty() {
            self.agent_session.into_iter().collect()
        } else {
            self.agent_sessions
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub(super) struct PaneFleetPersistedAgentSession {
    pub agent: CLIAgent,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_session_id: Option<String>,
    /// Which pane of the tab the agent was running in, as the terminal's
    /// persistent UUID in hex.
    ///
    /// Absent in files written before split layouts survived a restart, and
    /// absent whenever the pane could not be identified; restore then falls
    /// back to the active pane, which is what it always used to do.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pane_uuid: Option<String>,
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
            pane_uuid: None,
        }
    }

    pub fn set_pane_uuid(&mut self, pane_uuid: &[u8]) {
        self.pane_uuid = Some(encode_pane_uuid(pane_uuid));
    }

    pub fn pane_uuid_bytes(&self) -> Option<Vec<u8>> {
        decode_pane_uuid(self.pane_uuid.as_deref()?)
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
            | CLIAgent::WarpTui
            | CLIAgent::Unknown => Err(PaneFleetResumeError::UnsupportedAgent(self.agent)),
        }
    }
}

/// Hex rather than a byte array, so the state file stays readable by a person
/// and by whatever else reads it.
pub(super) fn encode_pane_uuid(pane_uuid: &[u8]) -> String {
    pane_uuid
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>()
}

fn decode_pane_uuid(encoded: &str) -> Option<Vec<u8>> {
    if encoded.is_empty() || !encoded.len().is_multiple_of(2) {
        return None;
    }
    encoded
        .as_bytes()
        .chunks(2)
        .map(|pair| {
            let pair = std::str::from_utf8(pair).ok()?;
            u8::from_str_radix(pair, 16).ok()
        })
        .collect()
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
