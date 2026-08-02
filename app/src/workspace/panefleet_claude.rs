#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum PaneFleetClaudeActivity {
    Tool { name: String },
    Skill { name: Option<String> },
    Subagent { name: Option<String> },
}

/// Classifies a tool invocation, naming the local capability when the plugin
/// forwarded it.
///
/// `capability` is the whitelisted `tool_input.skill` / `tool_input.subagent_type`.
/// Plugins that do not forward it yield an unnamed skill or subagent, which is
/// what every build did before the whitelist existed.
pub(crate) fn classify_claude_tool(
    tool_name: &str,
    capability: Option<&str>,
) -> Option<PaneFleetClaudeActivity> {
    let tool_name = compact_identifier(tool_name)?;
    let capability = capability.and_then(compact_identifier);
    match tool_name.as_str() {
        "Skill" => Some(PaneFleetClaudeActivity::Skill { name: capability }),
        "Agent" | "Task" => Some(PaneFleetClaudeActivity::Subagent { name: capability }),
        _ => Some(PaneFleetClaudeActivity::Tool { name: tool_name }),
    }
}

fn compact_identifier(identifier: &str) -> Option<String> {
    const MAX_CHARS: usize = 80;
    let identifier = identifier.trim();
    if identifier.is_empty() {
        return None;
    }
    if identifier.chars().count() <= MAX_CHARS {
        return Some(identifier.to_string());
    }
    let mut compact = identifier.chars().take(MAX_CHARS - 1).collect::<String>();
    compact.push('…');
    Some(compact)
}

#[cfg(test)]
#[path = "panefleet_claude_tests.rs"]
mod tests;
