# CODE-1827 PR 1: Centralized TUI Blocking State
## Context
The TUI already resolves and renders the concrete front-of-queue blocker inside `crates/warp_tui/src/agent_block.rs (63-87, 803-846)`, projects it through `crates/warp_tui/src/transcript_view.rs (606-613)`, and performs concrete focus transfer in `crates/warp_tui/src/terminal_session_view.rs (640-689)`. The session view also owns the input and footer whose visibility changes while an interaction blocks normal input.
## Changes
Define one `TuiBlockInputSource` enum in `crates/warp_tui/src/terminal_session_view.rs` for every input blocker that exists today: `LongRunningCommand`, `AskQuestion(ViewHandle<TuiAskQuestionView>)`, `Permission(ViewHandle<TuiPermissionPrompt>)`, and `Orchestration(ViewHandle<TuiOrchestrationBlock>)`. `TuiTerminalSessionView` stores `should_block_input: Option<TuiBlockInputSource>`. Action-queue, transcript, and terminal-process transitions reconcile the field, then update focus and notify the session view.

The view-backed variants retain the concrete focus target, so the same enum drives input suppression and focus without a parallel child type or category mapping. Concrete views retain rendering, placement, action status, and lifecycle behavior.
## Testing
Cover session input suppression and restoration from the session-owned blocking field. Retain the full existing ask-question, permission, orchestration, transcript, and focus suites.
