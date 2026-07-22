# CODE-1827 PR 1: Centralized TUI Blocking State
## Context
The TUI already resolves and renders the concrete front-of-queue blocker inside `crates/warp_tui/src/agent_block.rs (63-87, 803-846)`, projects it through `crates/warp_tui/src/transcript_view.rs (606-613)`, and performs concrete focus transfer in `crates/warp_tui/src/terminal_session_view.rs (640-689)`. The missing primitive is one authoritative, explicitly settable identity for whether normal session input is blocked.
## Changes
Add `crates/warp_tui/src/blocking_interaction.rs` with `TuiBlockingInteractionModel { blocker: Option<EntityId> }`.

The model owns only blocker identity and notifications:

- `activate(blocker)` succeeds when empty, is idempotent for the same blocker, and returns `TuiBlockerAlreadyActive` for a different blocker.
- `deactivate(blocker)` clears only matching ownership, so stale teardown cannot clear a newer blocker.
- `blocker()` and `is_active()` expose the centralized state.

Concrete views retain rendering, placement, focus, action status, and lifecycle behavior. `TuiTerminalSessionView` publishes the existing derived action blocker into the model and reads `is_active()` for input suppression. Existing `TuiBlockingChild` logic remains local to rendering and focus ownership.
## Testing
Cover strict activation, ordered deactivation/reactivation, stale deactivation, change notifications, and session input suppression/restoration. Retain the full existing ask-question, permission, orchestration, transcript, and focus suites.
