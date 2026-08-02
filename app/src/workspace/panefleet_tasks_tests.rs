use std::fs;
use std::path::PathBuf;

use tempfile::tempdir;

use super::{
    PANEFLEET_TASKS_VERSION, PaneFleetTaskBinding, PaneFleetTaskState, PaneFleetTaskStore,
};

fn binding(id: &str, input: &str) -> PaneFleetTaskBinding {
    PaneFleetTaskBinding::from_input(id.to_string(), input).expect("binding from input")
}

#[test]
fn splits_a_leading_tracker_key_off_the_title() {
    let task = binding("t-0001", "SEC-1802 Onboard Miro audit logs");

    assert_eq!(
        task.external.as_ref().map(|external| external.key.as_str()),
        Some("SEC-1802")
    );
    assert_eq!(task.title, "Onboard Miro audit logs");
    assert_eq!(task.label(), "SEC-1802 · Onboard Miro audit logs");
}

#[test]
fn recognizes_lowercase_and_multi_segment_keys() {
    assert_eq!(
        binding("t-0001", "inc-36884 Databricks PAT triage")
            .external
            .map(|external| external.key),
        Some("inc-36884".to_string())
    );
    assert_eq!(
        binding("t-0002", "leaver-2026-08 Departure review batch")
            .external
            .map(|external| external.key),
        Some("leaver-2026-08".to_string())
    );
}

#[test]
fn treats_a_leading_word_as_prose_not_as_a_key() {
    let task = binding("t-0001", "Onboard Miro audit logs");

    assert_eq!(task.external, None);
    assert_eq!(task.title, "Onboard Miro audit logs");
    assert_eq!(task.label(), "Onboard Miro audit logs");
}

#[test]
fn keeps_a_lone_key_as_the_title_so_it_is_never_lost() {
    let task = binding("t-0001", "SEC-1802");

    assert_eq!(task.external, None);
    assert_eq!(task.title, "SEC-1802");
}

#[test]
fn rejects_input_without_a_title() {
    assert!(PaneFleetTaskBinding::from_input("t-0001".to_string(), "   ").is_none());
}

#[test]
fn round_trips_input_text_for_editing() {
    let task = binding("t-0001", "SEC-1802 Onboard Miro audit logs");

    assert_eq!(task.input_text(), "SEC-1802 Onboard Miro audit logs");
    assert_eq!(binding("t-0001", &task.input_text()), task);
}

#[test]
fn applying_input_preserves_identity_and_work_state() {
    let mut task = binding("t-0007", "SEC-1802 Onboard Miro audit logs");
    task.state = PaneFleetTaskState::NeedsReview;
    task.done_check = Some(vec![
        "./deployment/deploy.sh".to_string(),
        "--dry-run".into(),
    ]);

    assert!(task.apply_input("SEC-1810 Onboard Figma audit logs"));

    assert_eq!(task.id, "t-0007");
    assert_eq!(task.state, PaneFleetTaskState::NeedsReview);
    assert!(task.done_check.is_some());
    assert_eq!(task.title, "Onboard Figma audit logs");
}

#[test]
fn survives_a_restart_and_stays_readable_by_another_process() {
    let directory = tempdir().expect("tempdir");
    let path = directory.path().join("panefleet-tasks.json");
    let environment = PathBuf::from("/tmp/sentinel-worktrees/feat-miro-connector");
    let mut store = PaneFleetTaskStore::default();
    store.set(
        environment.clone(),
        binding("t-0001", "SEC-1802 Onboard Miro audit logs"),
    );

    store.write_atomic(&path).expect("write task store");

    let json = fs::read_to_string(&path).expect("read task store");
    assert!(json.contains(r#""version": 1"#));
    assert!(json.contains(r#""key": "SEC-1802""#));
    // Fields the task cannot express yet must not be written as nulls.
    assert!(!json.contains("done_check"));
    assert!(!json.contains("template"));

    let loaded = PaneFleetTaskStore::load_or_default(&path);
    let loaded_task = loaded.get(&environment).expect("loaded task");
    assert_eq!(loaded_task.title, "Onboard Miro audit logs");
    assert_eq!(loaded_task.state, PaneFleetTaskState::Queued);
}

#[test]
fn ignores_a_store_from_a_newer_schema() {
    let directory = tempdir().expect("tempdir");
    let path = directory.path().join("panefleet-tasks.json");
    fs::write(
        &path,
        format!(
            r#"{{"version":{},"tasks":{{"/tmp/project":{{"id":"t-0001","title":"From the future"}}}}}}"#,
            PANEFLEET_TASKS_VERSION + 1
        ),
    )
    .expect("write future schema");

    let loaded = PaneFleetTaskStore::load_or_default(&path);

    assert_eq!(loaded.version, PANEFLEET_TASKS_VERSION);
    assert!(loaded.labels().is_empty());
}

#[test]
fn never_reuses_an_id_after_its_task_is_deleted() {
    let directory = tempdir().expect("tempdir");
    let path = directory.path().join("panefleet-tasks.json");
    let mut store = PaneFleetTaskStore::default();

    let first = store.allocate_id();
    let second = store.allocate_id();
    assert_eq!(first, "t-0001");
    assert_eq!(second, "t-0002");
    store.set(PathBuf::from("/tmp/a"), binding(&first, "First task"));
    store.set(PathBuf::from("/tmp/b"), binding(&second, "Second task"));

    store.remove(&PathBuf::from("/tmp/b"));
    assert_eq!(store.allocate_id(), "t-0003");

    // The high-water mark survives a restart.
    store.write_atomic(&path).expect("write task store");
    let mut reloaded = PaneFleetTaskStore::load_or_default(&path);
    assert_eq!(reloaded.allocate_id(), "t-0004");
}

#[test]
fn recovers_the_id_counter_from_a_hand_edited_file() {
    let directory = tempdir().expect("tempdir");
    let path = directory.path().join("panefleet-tasks.json");
    fs::write(
        &path,
        r#"{"version":1,"tasks":{"/tmp/project":{"id":"t-0042","title":"Added by hand"}}}"#,
    )
    .expect("write hand-edited store");

    let mut loaded = PaneFleetTaskStore::load_or_default(&path);

    assert_eq!(loaded.allocate_id(), "t-0043");
}

#[test]
fn refuses_to_overwrite_a_file_it_could_not_read() {
    let directory = tempdir().expect("tempdir");
    let path = directory.path().join("panefleet-tasks.json");
    fs::write(&path, "{ this is not json").expect("write corrupt store");

    let mut loaded = PaneFleetTaskStore::load_or_default(&path);
    loaded.set(PathBuf::from("/tmp/a"), binding("t-0001", "A new task"));

    // Writing here would turn a read failure into permanent loss of whatever
    // the unreadable file actually held.
    assert!(loaded.write_atomic(&path).is_err());
    assert_eq!(
        fs::read_to_string(&path).expect("file survives"),
        "{ this is not json"
    );
}

#[test]
fn a_missing_file_is_writable_because_nothing_can_be_lost() {
    let directory = tempdir().expect("tempdir");
    let path = directory.path().join("panefleet-tasks.json");

    let mut store = PaneFleetTaskStore::load_or_default(&path);
    store.set(PathBuf::from("/tmp/a"), binding("t-0001", "A new task"));

    assert!(store.write_atomic(&path).is_ok());
}

#[test]
fn a_future_schema_is_left_alone_rather_than_replaced() {
    let directory = tempdir().expect("tempdir");
    let path = directory.path().join("panefleet-tasks.json");
    let future = format!(
        r#"{{"version":{},"tasks":{{}}}}"#,
        PANEFLEET_TASKS_VERSION + 1
    );
    fs::write(&path, &future).expect("write future schema");

    let mut loaded = PaneFleetTaskStore::load_or_default(&path);
    loaded.set(PathBuf::from("/tmp/a"), binding("t-0001", "A new task"));

    assert!(loaded.write_atomic(&path).is_err());
    assert_eq!(fs::read_to_string(&path).expect("file survives"), future);
}

#[test]
fn drops_bindings_whose_environment_is_gone() {
    let directory = tempdir().expect("tempdir");
    let alive = directory.path().join("alive");
    fs::create_dir(&alive).expect("create environment");
    let gone = directory.path().join("gone");

    let mut store = PaneFleetTaskStore::default();
    store.set(alive.clone(), binding("t-0001", "Still here"));
    store.set(gone.clone(), binding("t-0002", "Removed from the shell"));

    assert!(store.prune_missing_environments());
    assert!(store.get(&alive).is_some());
    assert!(store.get(&gone).is_none());
    // Ids are not recycled just because a binding went away.
    assert_eq!(store.allocate_id(), "t-0003");

    assert!(!store.prune_missing_environments());
}

#[test]
fn a_gate_round_trips_through_a_shell_command_line() {
    let mut task = binding("t-0001", "Onboard Miro audit logs");
    assert_eq!(task.done_check_text(), "");

    assert!(task.set_done_check_from_text("./deployment/deploy.sh --dry-run"));
    assert_eq!(
        task.done_check,
        Some(vec![
            "./deployment/deploy.sh".to_string(),
            "--dry-run".to_string()
        ])
    );
    assert_eq!(task.done_check_text(), "./deployment/deploy.sh --dry-run");

    // Quoting survives the round trip rather than splitting the argument.
    assert!(task.set_done_check_from_text("selfcheck.sh 'inc 36884'"));
    assert_eq!(
        task.done_check,
        Some(vec!["selfcheck.sh".to_string(), "inc 36884".to_string()])
    );
}

#[test]
fn an_empty_gate_clears_it_and_an_unparseable_one_is_refused() {
    let mut task = binding("t-0001", "Onboard Miro audit logs");
    assert!(task.set_done_check_from_text("cargo test"));

    assert!(task.set_done_check_from_text("   "));
    assert_eq!(task.done_check, None);

    assert!(task.set_done_check_from_text("cargo test"));
    // An unbalanced quote must not silently drop the gate that is already set.
    assert!(!task.set_done_check_from_text("cargo test 'unterminated"));
    assert_eq!(task.done_check_text(), "cargo test");
}

#[test]
fn the_gate_decides_between_review_and_confirmation() {
    let mut task = binding("t-0001", "Onboard Miro audit logs");
    assert_eq!(task.state, PaneFleetTaskState::Queued);

    task.apply_done_check_outcome(false);
    assert_eq!(task.state, PaneFleetTaskState::NeedsReview);

    // Never `Done` — that word belongs to the person, not the check.
    task.apply_done_check_outcome(true);
    assert_eq!(task.state, PaneFleetTaskState::AwaitingAck);
}

#[test]
fn a_passing_gate_asks_for_confirmation_rather_than_claiming_done() {
    let mut task = binding("t-0001", "Onboard Miro audit logs");

    task.apply_done_check_outcome(true);

    // The check passing is evidence for a person, not a substitute for them.
    assert_eq!(task.state, PaneFleetTaskState::AwaitingAck);
    assert_eq!(task.completed_at_unix_ms, None);
}

#[test]
fn only_a_person_reaches_done_and_the_moment_is_recorded() {
    let mut task = binding("t-0001", "Onboard Miro audit logs");
    task.apply_done_check_outcome(true);

    task.mark_done(1_700_000_000_000);

    assert_eq!(task.state, PaneFleetTaskState::Done);
    assert_eq!(task.completed_at_unix_ms, Some(1_700_000_000_000));
    // A passing gate stood behind this one.
    assert!(!task.completed_without_gate);
}

#[test]
fn confirming_without_a_passing_gate_is_recorded_as_such() {
    for state in [
        PaneFleetTaskState::Queued,
        PaneFleetTaskState::Working,
        PaneFleetTaskState::NeedsReview,
    ] {
        let mut task = binding("t-0001", "Onboard Miro audit logs");
        task.state = state;

        task.mark_done(1_700_000_000_000);

        assert_eq!(task.state, PaneFleetTaskState::Done);
        // Nothing checked the work, so a card must not present it as checked.
        assert!(task.completed_without_gate, "{state:?} should be flagged");
    }
}

#[test]
fn a_new_turn_after_confirmation_clears_the_completion() {
    let mut task = binding("t-0001", "Onboard Miro audit logs");
    task.apply_done_check_outcome(true);
    task.mark_done(1_700_000_000_000);

    // Refining a finished task is the common case, not an exception.
    task.apply_done_check_outcome(false);

    assert_eq!(task.state, PaneFleetTaskState::NeedsReview);
    assert_eq!(task.completed_at_unix_ms, None);
    assert!(!task.completed_without_gate);
}
