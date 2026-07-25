use super::{
    PANEFLEET_AGENT_DEFINITIONS_VERSION, PaneFleetAgentDefinition, PaneFleetAgentDefinitions,
    PaneFleetPromptTransport,
};
use crate::terminal::CLIAgent;

#[test]
fn bundled_launchers_have_stable_order() {
    let definitions = PaneFleetAgentDefinitions::bundled_defaults();

    assert_eq!(
        definitions
            .enabled_launchers()
            .into_iter()
            .map(|definition| definition.id.as_str())
            .collect::<Vec<_>>(),
        ["builtin.codex", "builtin.claude", "builtin.opencode"]
    );
}

#[test]
fn codex_launch_does_not_inherit_parent_conversation() {
    let definitions = PaneFleetAgentDefinitions::bundled_defaults();
    let codex = definitions.first_for_agent(CLIAgent::Codex).unwrap();

    assert_eq!(
        codex.launch_command([]),
        Some("env -u CODEX_THREAD_ID -u CODEX_CI codex".to_string())
    );
}

#[test]
fn launch_command_quotes_structured_arguments() {
    let definition = PaneFleetAgentDefinition {
        id: "custom.agent".to_string(),
        label: "Custom".to_string(),
        agent: CLIAgent::Unknown,
        executable: "/Applications/My Agent/bin/agent".to_string(),
        args: vec!["--profile".to_string(), "careful mode".to_string()],
        prompt_only_args: Vec::new(),
        prompt_transport: PaneFleetPromptTransport::Argv,
        enabled_in_launcher: true,
        launcher_order: 1,
        bundled: false,
    };

    assert_eq!(
        definition.launch_command(["--prompt", "review this"]),
        Some(
            "'/Applications/My Agent/bin/agent' --profile 'careful mode' --prompt 'review this'"
                .to_string()
        )
    );
}

#[test]
fn definitions_round_trip_through_versioned_json() {
    let definitions = PaneFleetAgentDefinitions::bundled_defaults();
    let json = serde_json::to_vec(&definitions).unwrap();
    let decoded = PaneFleetAgentDefinitions::decode(&json).unwrap();

    assert_eq!(decoded.version, PANEFLEET_AGENT_DEFINITIONS_VERSION);
    assert_eq!(decoded, definitions);
}
