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
