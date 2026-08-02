use super::{
    PaneFleetAttentionColumn, PaneFleetAttentionCounts, PaneFleetAttentionInput, attention_column,
    attention_reason,
};
use crate::workspace::panefleet_tasks::PaneFleetTaskState;

fn task(state: PaneFleetTaskState) -> PaneFleetAttentionInput {
    PaneFleetAttentionInput {
        task_state: Some(state),
        ..Default::default()
    }
}

#[test]
fn a_failed_check_asks_to_be_thought_about() {
    assert_eq!(
        attention_column(task(PaneFleetTaskState::NeedsReview)),
        Some(PaneFleetAttentionColumn::Unblock)
    );
    assert_eq!(
        attention_reason(task(PaneFleetTaskState::NeedsReview)),
        "check did not pass"
    );
}

#[test]
fn a_passed_check_asks_to_be_confirmed() {
    assert_eq!(
        attention_column(task(PaneFleetTaskState::AwaitingAck)),
        Some(PaneFleetAttentionColumn::Authorize)
    );
}

#[test]
fn a_blocked_agent_outranks_whatever_the_work_axis_last_said() {
    for state in [
        PaneFleetTaskState::Queued,
        PaneFleetTaskState::Working,
        PaneFleetTaskState::AwaitingAck,
        PaneFleetTaskState::Done,
    ] {
        let input = PaneFleetAttentionInput {
            agent_is_blocked: true,
            ..task(state)
        };
        assert_eq!(
            attention_column(input),
            Some(PaneFleetAttentionColumn::Unblock),
            "{state:?} with a blocked agent is stuck until answered"
        );
        assert_eq!(attention_reason(input), "agent is asking");
    }
}

#[test]
fn a_failed_run_means_the_work_is_not_done() {
    let input = PaneFleetAttentionInput {
        agent_failed: true,
        ..task(PaneFleetTaskState::AwaitingAck)
    };

    assert_eq!(
        attention_column(input),
        Some(PaneFleetAttentionColumn::Unblock)
    );
    assert_eq!(attention_reason(input), "agent run failed");
}

#[test]
fn work_underway_is_not_waiting_to_be_pulled() {
    assert_eq!(
        attention_column(task(PaneFleetTaskState::Queued)),
        Some(PaneFleetAttentionColumn::Pull)
    );

    // An agent is already on it, so there is no decision to make.
    let running = PaneFleetAttentionInput {
        agent_is_working: true,
        ..task(PaneFleetTaskState::Queued)
    };
    assert_eq!(attention_column(running), None);
}

#[test]
fn states_that_ask_nothing_get_no_column() {
    assert_eq!(attention_column(task(PaneFleetTaskState::Working)), None);
    assert_eq!(attention_column(task(PaneFleetTaskState::Done)), None);
    assert_eq!(
        attention_column(PaneFleetAttentionInput::default()),
        None,
        "an environment with no task is not on the board"
    );
}

#[test]
fn the_strip_counts_what_has_no_column() {
    let mut counts = PaneFleetAttentionCounts::default();
    counts.add(task(PaneFleetTaskState::Working), false);
    counts.add(task(PaneFleetTaskState::NeedsReview), false);
    counts.add(task(PaneFleetTaskState::AwaitingAck), false);
    counts.add(task(PaneFleetTaskState::Queued), false);
    counts.add(task(PaneFleetTaskState::Done), true);
    counts.add(task(PaneFleetTaskState::Done), false);

    assert_eq!(
        counts,
        PaneFleetAttentionCounts {
            working: 1,
            // Unblock plus Authorize.
            needs_you: 2,
            // The two `Done` tasks; `Queued` has its own column instead.
            quiet: 2,
            // Only the one finished inside the window.
            done_today: 1,
        }
    );
}

#[test]
fn an_environment_with_no_task_is_not_counted_at_all() {
    let mut counts = PaneFleetAttentionCounts::default();
    counts.add(PaneFleetAttentionInput::default(), false);
    counts.add(
        PaneFleetAttentionInput {
            agent_is_working: true,
            ..Default::default()
        },
        false,
    );

    assert_eq!(counts, PaneFleetAttentionCounts::default());
}
