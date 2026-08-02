use super::{PaneFleetClaudeActivity, classify_claude_tool};

#[test]
fn names_the_skill_and_subagent_when_the_plugin_forwards_it() {
    assert_eq!(
        classify_claude_tool("Skill", Some("sentinel-investigate")),
        Some(PaneFleetClaudeActivity::Skill {
            name: Some("sentinel-investigate".to_string())
        })
    );
    assert_eq!(
        classify_claude_tool("Agent", Some("Explore")),
        Some(PaneFleetClaudeActivity::Subagent {
            name: Some("Explore".to_string())
        })
    );
    assert_eq!(
        classify_claude_tool("Task", Some("code-reviewer")),
        Some(PaneFleetClaudeActivity::Subagent {
            name: Some("code-reviewer".to_string())
        })
    );
}

#[test]
fn falls_back_to_an_unnamed_capability_without_guessing() {
    // Plugins that do not forward the name — every build before the whitelist.
    assert_eq!(
        classify_claude_tool("Skill", None),
        Some(PaneFleetClaudeActivity::Skill { name: None })
    );
    assert_eq!(
        classify_claude_tool("Agent", Some("   ")),
        Some(PaneFleetClaudeActivity::Subagent { name: None })
    );
}

#[test]
fn preserves_normal_and_mcp_tool_names() {
    assert_eq!(
        classify_claude_tool("Read", None),
        Some(PaneFleetClaudeActivity::Tool {
            name: "Read".to_string()
        })
    );
    assert_eq!(
        classify_claude_tool("mcp__github__get_pull_request", None),
        Some(PaneFleetClaudeActivity::Tool {
            name: "mcp__github__get_pull_request".to_string()
        })
    );
}

#[test]
fn a_capability_name_is_ignored_for_tools_that_do_not_have_one() {
    assert_eq!(
        classify_claude_tool("Read", Some("not-a-skill")),
        Some(PaneFleetClaudeActivity::Tool {
            name: "Read".to_string()
        })
    );
}

#[test]
fn rejects_empty_names_and_bounds_untrusted_plugin_metadata() {
    assert_eq!(classify_claude_tool("   ", None), None);
    let activity = classify_claude_tool(&"x".repeat(120), None).expect("bounded tool");
    let PaneFleetClaudeActivity::Tool { name } = activity else {
        panic!("expected a regular tool");
    };
    assert_eq!(name.chars().count(), 80);
    assert!(name.ends_with('…'));

    // The capability name is plugin-supplied too, so it is bounded the same way.
    let activity =
        classify_claude_tool("Skill", Some(&"y".repeat(120))).expect("bounded capability");
    let PaneFleetClaudeActivity::Skill { name } = activity else {
        panic!("expected a skill");
    };
    let name = name.expect("named skill");
    assert_eq!(name.chars().count(), 80);
    assert!(name.ends_with('…'));
}
