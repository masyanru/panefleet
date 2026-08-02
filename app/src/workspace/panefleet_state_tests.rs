use std::path::PathBuf;

use uuid::Uuid;

use super::{
    PANEFLEET_STATE_VERSION, PaneFleetPersistedAgentSession, PaneFleetPersistedState,
    PaneFleetResumeError, PaneFleetWorkspaceSource, panefleet_agent_launch_command,
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
    assert_eq!(
        state.workspaces[0].source,
        PaneFleetWorkspaceSource::ExistingFolder
    );
    assert_eq!(state.workspaces[0].tabs[0].agent_session, None);
}

#[test]
fn remembers_which_pane_the_agent_ran_in() {
    let mut session = PaneFleetPersistedAgentSession::new(CLIAgent::Claude, None);
    assert_eq!(session.pane_uuid_bytes(), None);

    let pane_uuid = vec![0x00, 0x0f, 0xa0, 0xff];
    session.set_pane_uuid(&pane_uuid);

    assert_eq!(session.pane_uuid.as_deref(), Some("000fa0ff"));
    assert_eq!(session.pane_uuid_bytes(), Some(pane_uuid));
}

#[test]
fn a_state_file_without_a_pane_address_still_loads() {
    // Written before split layouts survived a restart. Such a session must
    // restore into the active pane rather than be dropped.
    let legacy = br#"{
        "version": 3,
        "active_project": "/tmp/project",
        "workspaces": [{
            "path": "/tmp/project",
            "active_tab_index": 0,
            "tabs": [{
                "title": "project",
                "agent_session": {"agent": "Claude", "provider_session_id": "abc"}
            }]
        }]
    }"#;

    let state = PaneFleetPersistedState::decode(legacy).unwrap();
    let session = state.workspaces[0].tabs[0]
        .agent_session
        .as_ref()
        .expect("agent session survives");

    assert_eq!(session.agent, CLIAgent::Claude);
    assert_eq!(session.pane_uuid, None);
    assert_eq!(session.pane_uuid_bytes(), None);
}

#[test]
fn a_tab_keeps_one_session_per_pane() {
    let raw = br#"{
        "version": 3,
        "active_project": "/tmp/project",
        "workspaces": [{
            "path": "/tmp/project",
            "active_tab_index": 0,
            "tabs": [{
                "title": "SEC-1802 Onboard Miro audit logs",
                "agent_sessions": [
                    {"agent": "Claude", "provider_session_id": "left", "pane_uuid": "aa01"},
                    {"agent": "Claude", "provider_session_id": "right", "pane_uuid": "bb02"}
                ]
            }]
        }]
    }"#;

    let mut state = PaneFleetPersistedState::decode(raw).unwrap();
    let sessions = state.workspaces.remove(0).tabs.remove(0).sessions();

    assert_eq!(sessions.len(), 2);
    assert_eq!(
        sessions
            .iter()
            .map(|session| session.pane_uuid.as_deref())
            .collect::<Vec<_>>(),
        [Some("aa01"), Some("bb02")]
    );
    // Two agents in one tab must not collapse into one another.
    assert_ne!(
        sessions[0].provider_session_id,
        sessions[1].provider_session_id
    );
}

#[test]
fn a_legacy_single_session_tab_still_restores_its_agent() {
    let legacy = br#"{
        "version": 3,
        "active_project": "/tmp/project",
        "workspaces": [{
            "path": "/tmp/project",
            "active_tab_index": 0,
            "tabs": [{
                "title": "project",
                "agent_session": {"agent": "Claude", "provider_session_id": "only"}
            }]
        }]
    }"#;

    let mut state = PaneFleetPersistedState::decode(legacy).unwrap();
    let sessions = state.workspaces.remove(0).tabs.remove(0).sessions();

    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].provider_session_id.as_deref(), Some("only"));
    assert_eq!(sessions[0].pane_uuid, None);
}

#[test]
fn a_damaged_pane_address_is_ignored_rather_than_trusted() {
    for damaged in ["", "abc", "zz", "00ff0"] {
        let session = PaneFleetPersistedAgentSession {
            agent: CLIAgent::Claude,
            provider_session_id: None,
            pane_uuid: Some(damaged.to_string()),
        };

        assert_eq!(
            session.pane_uuid_bytes(),
            None,
            "'{damaged}' should not decode"
        );
    }
}

#[test]
fn round_trips_isolated_worktree_metadata() {
    let encoded = br#"{
        "version": 3,
        "active_project": "/tmp/worktrees/feature",
        "workspaces": [{
            "path": "/tmp/worktrees/feature",
            "source": {
                "kind": "isolated_worktree",
                "source_repository": "/tmp/project",
                "branch": "feature/panefleet",
                "managed": true
            },
            "tabs": []
        }]
    }"#;

    let state = PaneFleetPersistedState::decode(encoded).unwrap();

    assert_eq!(
        state.workspaces[0].source,
        PaneFleetWorkspaceSource::IsolatedWorktree {
            source_repository: PathBuf::from("/tmp/project"),
            branch: "feature/panefleet".to_string(),
            managed: true,
        }
    );
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

#[test]
fn rejects_warp_tui_resume_without_an_adapter() {
    let session = PaneFleetPersistedAgentSession::new(
        CLIAgent::WarpTui,
        Some("warp-tui-session".to_string()),
    );

    assert_eq!(
        session.resume_command(),
        Err(PaneFleetResumeError::UnsupportedAgent(CLIAgent::WarpTui))
    );
}
