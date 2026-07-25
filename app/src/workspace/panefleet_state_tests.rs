use std::path::PathBuf;

use uuid::Uuid;

use super::{
    PANEFLEET_STATE_VERSION, PaneFleetPersistedAgentSession, PaneFleetPersistedState,
    PaneFleetResumeError, panefleet_agent_launch_command,
};
use crate::terminal::CLIAgent;

#[test]
fn migrates_legacy_state_without_agent_metadata() {
    let legacy = br#"{
        "active_project": "/tmp/project",
        "workspaces": [{
            "path": "/tmp/project",
            "active_tab_index": 0,
            "tabs": [{"title": "project"}]
        }]
    }"#;

    let state = PaneFleetPersistedState::decode(legacy).unwrap();

    assert_eq!(state.version, PANEFLEET_STATE_VERSION);
    assert_eq!(state.active_project, Some(PathBuf::from("/tmp/project")));
    assert_eq!(state.workspaces[0].tabs[0].agent_session, None);
}

#[test]
fn builds_claude_resume_command_from_provider_session_id() {
    let session_id = Uuid::new_v4();
    let session =
        PaneFleetPersistedAgentSession::new(CLIAgent::Claude, Some(session_id.to_string()));

    assert_eq!(
        session.resume_command(),
        Ok(format!("claude --resume {session_id}"))
    );
}

#[test]
fn builds_codex_resume_command_from_provider_session_id() {
    let session_id = Uuid::new_v4();
    let session =
        PaneFleetPersistedAgentSession::new(CLIAgent::Codex, Some(session_id.to_string()));

    assert_eq!(
        session.resume_command(),
        Ok(format!(
            "env -u CODEX_THREAD_ID -u CODEX_CI codex resume {session_id}"
        ))
    );
}

#[test]
fn isolates_new_codex_session_from_parent_codex_runtime() {
    assert_eq!(
        panefleet_agent_launch_command(CLIAgent::Codex),
        "env -u CODEX_THREAD_ID -u CODEX_CI codex"
    );
}

#[test]
fn refuses_to_start_fresh_session_when_resume_id_is_missing() {
    let session = PaneFleetPersistedAgentSession::new(CLIAgent::Claude, None);

    assert_eq!(
        session.resume_command(),
        Err(PaneFleetResumeError::MissingSessionId)
    );
}

#[test]
fn builds_opencode_resume_command_from_opaque_session_id() {
    let session = PaneFleetPersistedAgentSession::new(
        CLIAgent::OpenCode,
        Some("ses_06f58746affe9XsetR90cbxo7r".to_string()),
    );

    assert_eq!(
        session.resume_command(),
        Ok("opencode -s ses_06f58746affe9XsetR90cbxo7r".to_string())
    );
}

#[test]
fn quotes_opencode_session_id_before_building_shell_command() {
    let session = PaneFleetPersistedAgentSession::new(
        CLIAgent::OpenCode,
        Some("session with spaces".to_string()),
    );

    assert_eq!(
        session.resume_command(),
        Ok("opencode -s 'session with spaces'".to_string())
    );
}
