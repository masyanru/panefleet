use super::{
    PANEFLEET_AGENT_DEFINITIONS_VERSION, PaneFleetAgentDefinition, PaneFleetAgentDefinitions,
    PaneFleetPromptTransport, single_instance_conflict,
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
        single_instance_per_machine: false,
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

fn prompting_definition(transport: PaneFleetPromptTransport) -> PaneFleetAgentDefinition {
    PaneFleetAgentDefinition {
        id: "custom.agent".to_string(),
        label: "Custom".to_string(),
        agent: CLIAgent::Claude,
        executable: "claude".to_string(),
        args: vec!["--permission-mode".to_string(), "plan".to_string()],
        prompt_only_args: vec!["-p".to_string()],
        prompt_transport: transport,
        single_instance_per_machine: false,
        enabled_in_launcher: true,
        launcher_order: 1,
        bundled: false,
    }
}

#[test]
fn argv_transport_appends_the_prompt_after_prompt_only_arguments() {
    let definition = prompting_definition(PaneFleetPromptTransport::Argv);

    assert_eq!(
        definition.launch_command_with_prompt("Onboard Miro audit logs", ["--session-id", "abc"]),
        Some(
            "claude --permission-mode plan -p --session-id abc 'Onboard Miro audit logs'"
                .to_string()
        )
    );
}

#[test]
fn stdin_transport_pipes_the_prompt_without_letting_the_shell_read_it() {
    let definition = prompting_definition(PaneFleetPromptTransport::Stdin);

    assert_eq!(
        definition.launch_command_with_prompt("--not-a-flag $HOME", []),
        Some("printf '%s' '--not-a-flag $HOME' | claude --permission-mode plan -p".to_string())
    );
}

#[test]
fn a_blank_prompt_launches_the_agent_normally() {
    for transport in [
        PaneFleetPromptTransport::Argv,
        PaneFleetPromptTransport::Stdin,
    ] {
        let definition = prompting_definition(transport);

        // `-p` would put the agent in one-shot mode with nothing to work on.
        assert_eq!(
            definition.launch_command_with_prompt("   ", []),
            Some("claude --permission-mode plan".to_string())
        );
    }
}

#[test]
fn prompt_only_arguments_stay_out_of_a_plain_launch() {
    let definition = prompting_definition(PaneFleetPromptTransport::Argv);

    assert_eq!(
        definition.launch_command([]),
        Some("claude --permission-mode plan".to_string())
    );
}

#[test]
fn codex_refuses_a_second_copy_and_says_why() {
    let definitions = PaneFleetAgentDefinitions::bundled_defaults();
    let codex = definitions.first_for_agent(CLIAgent::Codex).unwrap();

    assert_eq!(single_instance_conflict(codex, []), None);
    assert_eq!(
        single_instance_conflict(codex, [CLIAgent::Claude, CLIAgent::OpenCode]),
        None
    );

    let message = single_instance_conflict(codex, [CLIAgent::Claude, CLIAgent::Codex])
        .expect("second Codex is refused");
    assert!(message.contains("Codex"));
    // The reason has to be in the message; "not allowed" teaches nothing.
    assert!(message.contains("shared file"));
}

#[test]
fn agents_without_the_constraint_may_run_side_by_side() {
    let definitions = PaneFleetAgentDefinitions::bundled_defaults();

    for agent in [CLIAgent::Claude, CLIAgent::OpenCode] {
        let definition = definitions.first_for_agent(agent).unwrap();
        assert_eq!(single_instance_conflict(definition, [agent, agent]), None);
    }
}

#[test]
fn definitions_round_trip_through_versioned_json() {
    let definitions = PaneFleetAgentDefinitions::bundled_defaults();
    let json = serde_json::to_vec(&definitions).unwrap();
    let decoded = PaneFleetAgentDefinitions::decode(&json).unwrap();

    assert_eq!(decoded.version, PANEFLEET_AGENT_DEFINITIONS_VERSION);
    assert_eq!(decoded, definitions);
}
