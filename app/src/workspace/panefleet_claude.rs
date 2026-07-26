#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum PaneFleetClaudeActivity {
    Tool { name: String },
    Skill { name: Option<String> },
    Subagent { name: Option<String> },
}

pub(crate) fn classify_claude_tool(tool_name: &str) -> Option<PaneFleetClaudeActivity> {
    let tool_name = compact_identifier(tool_name)?;
    match tool_name.as_str() {
        "Skill" => Some(PaneFleetClaudeActivity::Skill { name: None }),
        "Agent" | "Task" => Some(PaneFleetClaudeActivity::Subagent { name: None }),
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
