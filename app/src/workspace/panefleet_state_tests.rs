use std::path::PathBuf;

use uuid::Uuid;

use super::{
    PANEFLEET_STATE_VERSION, PaneFleetPersistedAgentSession, PaneFleetPersistedState,
    PaneFleetResumeError,
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
        Ok(format!("codex resume {session_id}"))
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
fn reports_unsupported_agent_instead_of_starting_it_again() {
    let session =
        PaneFleetPersistedAgentSession::new(CLIAgent::OpenCode, Some(Uuid::new_v4().to_string()));

    assert_eq!(
        session.resume_command(),
        Err(PaneFleetResumeError::UnsupportedAgent(CLIAgent::OpenCode))
    );
}
