use super::{PaneFleetClaudeActivity, classify_claude_tool};

#[test]
fn classifies_claude_skill_and_subagent_tools_without_guessing_names() {
    assert_eq!(
        classify_claude_tool("Skill"),
        Some(PaneFleetClaudeActivity::Skill { name: None })
    );
    assert_eq!(
        classify_claude_tool("Agent"),
        Some(PaneFleetClaudeActivity::Subagent { name: None })
    );
    assert_eq!(
        classify_claude_tool("Task"),
        Some(PaneFleetClaudeActivity::Subagent { name: None })
    );
}

#[test]
fn preserves_normal_and_mcp_tool_names() {
    assert_eq!(
        classify_claude_tool("Read"),
        Some(PaneFleetClaudeActivity::Tool {
            name: "Read".to_string()
        })
    );
    assert_eq!(
        classify_claude_tool("mcp__github__get_pull_request"),
        Some(PaneFleetClaudeActivity::Tool {
            name: "mcp__github__get_pull_request".to_string()
        })
    );
}

#[test]
fn rejects_empty_names_and_bounds_untrusted_plugin_metadata() {
    assert_eq!(classify_claude_tool("   "), None);
    let activity = classify_claude_tool(&"x".repeat(120)).expect("bounded tool");
    let PaneFleetClaudeActivity::Tool { name } = activity else {
        panic!("expected a regular tool");
    };
    assert_eq!(name.chars().count(), 80);
    assert!(name.ends_with('…'));
}
