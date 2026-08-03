//! Sorting bound work into what it asks of a person.
//!
//! The Fleet board groups by **type of action**, not by stage of a process.
//! A column earns its place only if it changes what someone does next, so
//! `Working` and `Done` get no column: one asks nothing, the other is over.
//! They are counted instead.

use super::panefleet_tasks::PaneFleetTaskState;

/// One day, the window the board reports finished work over.
pub(super) const DONE_WINDOW_MS: u64 = 24 * 60 * 60 * 1000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum PaneFleetAttentionColumn {
    /// Decide what to start. Exists while work is pulled by hand.
    Pull,
    /// The work is **not** done: an agent is asking, or the gate failed.
    /// Thought about one at a time.
    Unblock,
    /// The work passed its check and waits for a person to confirm it.
    /// Reviewed in batches, so several can be confirmed in one pass.
    Authorize,
}

/// What is known about one environment when deciding where it belongs.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct PaneFleetAttentionInput {
    pub task_state: Option<PaneFleetTaskState>,
    /// An agent is waiting on an answer — a permission request or a question.
    pub agent_is_blocked: bool,
    /// An agent is mid-turn.
    pub agent_is_working: bool,
    /// An agent run ended in failure.
    pub agent_failed: bool,
}

/// Which column an environment belongs in, or `None` when it asks nothing.
///
/// A blocked agent outranks everything: it is stuck until answered, whatever
/// the work axis last said. A failed gate and a failed run both mean the work
/// is not done and needs thought.
pub(super) fn attention_column(input: PaneFleetAttentionInput) -> Option<PaneFleetAttentionColumn> {
    if input.agent_is_blocked || input.agent_failed {
        return Some(PaneFleetAttentionColumn::Unblock);
    }
    match input.task_state {
        Some(PaneFleetTaskState::NeedsReview) => Some(PaneFleetAttentionColumn::Unblock),
        Some(PaneFleetTaskState::AwaitingAck) => Some(PaneFleetAttentionColumn::Authorize),
        // Nothing has been started here yet. An agent already working on it is
        // not waiting for that decision.
        Some(PaneFleetTaskState::Queued) if !input.agent_is_working => {
            Some(PaneFleetAttentionColumn::Pull)
        }
        // `Working` asks nothing; `Done` is over; an environment with no task
        // is not on the board at all.
        _ => None,
    }
}

/// Why an environment is where it is, for the line under its title.
pub(super) fn attention_reason(input: PaneFleetAttentionInput) -> &'static str {
    if input.agent_is_blocked {
        return "agent is asking";
    }
    if input.agent_failed {
        return "agent run failed";
    }
    match input.task_state {
        Some(PaneFleetTaskState::NeedsReview) => "check did not pass",
        Some(PaneFleetTaskState::AwaitingAck) => "check passed",
        Some(PaneFleetTaskState::Queued) => "not started",
        Some(PaneFleetTaskState::Working) => "working",
        Some(PaneFleetTaskState::Done) => "done",
        None => "no task",
    }
}

/// The counter strip.
///
/// `working`, `needs_you`, `to_pull` and `quiet` **partition** the bound work:
/// every task lands in exactly one. Tiles sitting side by side read as a
/// breakdown, so overlapping them would double-report the same task.
///
/// `finished_recently` deliberately stands outside that partition — finished
/// work is also quiet — so it is reported on its own line rather than as a
/// fifth tile.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct PaneFleetAttentionCounts {
    pub working: usize,
    pub needs_you: usize,
    pub to_pull: usize,
    /// Bound work that asks nothing right now — neither running nor waiting.
    pub quiet: usize,
    pub finished_recently: usize,
}

impl PaneFleetAttentionCounts {
    /// Counts one environment. Environments with no task are skipped: the strip
    /// reports on work, and an unnamed folder is not work yet.
    pub fn add(&mut self, input: PaneFleetAttentionInput, finished_recently: bool) {
        if input.task_state.is_none() {
            return;
        }
        if finished_recently {
            self.finished_recently += 1;
        }
        match attention_column(input) {
            Some(PaneFleetAttentionColumn::Unblock | PaneFleetAttentionColumn::Authorize) => {
                self.needs_you += 1;
            }
            Some(PaneFleetAttentionColumn::Pull) => self.to_pull += 1,
            None if input.agent_is_working
                || input.task_state == Some(PaneFleetTaskState::Working) =>
            {
                self.working += 1;
            }
            None => self.quiet += 1,
        }
    }
}

#[cfg(test)]
#[path = "panefleet_attention_tests.rs"]
mod tests;
