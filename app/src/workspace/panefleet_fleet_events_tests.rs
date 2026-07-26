use std::fs;

use tempfile::tempdir;
use uuid::Uuid;

use super::{
    MAX_FLEET_EVENTS, PANEFLEET_FLEET_EVENTS_VERSION, PaneFleetFleetEvent, PaneFleetFleetEventKind,
    PaneFleetFleetEventStore,
};
use crate::terminal::CLIAgent;

fn event(index: usize) -> PaneFleetFleetEvent {
    PaneFleetFleetEvent {
        id: Uuid::new_v4(),
        occurred_at_unix_ms: index as u64,
        workspace_path: format!("/tmp/project-{index}").into(),
        agent: CLIAgent::Codex,
        kind: PaneFleetFleetEventKind::Started,
        terminal_view_id: None,
    }
}

#[test]
fn retains_only_the_newest_events() {
    let mut store = PaneFleetFleetEventStore::default();
    for index in 0..MAX_FLEET_EVENTS + 7 {
        store.record(event(index));
    }

    let timestamps = store
        .recent()
        .map(|event| event.occurred_at_unix_ms)
        .collect::<Vec<_>>();
    assert_eq!(timestamps.len(), MAX_FLEET_EVENTS);
    assert_eq!(timestamps.first(), Some(&((MAX_FLEET_EVENTS + 6) as u64)));
    assert_eq!(timestamps.last(), Some(&7));
}

#[test]
fn persisted_events_do_not_include_process_local_entity_ids() {
    let directory = tempdir().expect("tempdir");
    let path = directory.path().join("fleet-events.json");
    let mut store = PaneFleetFleetEventStore::default();
    store.record(event(42));

    store.write_atomic(&path).expect("write event store");
    let json = fs::read_to_string(&path).expect("read event store");
    assert!(!json.contains("terminal_view_id"));

    let loaded = PaneFleetFleetEventStore::load_or_default(&path);
    let loaded_event = loaded.recent().next().expect("loaded event");
    assert_eq!(loaded_event.occurred_at_unix_ms, 42);
    assert_eq!(loaded_event.terminal_view_id, None);
}

#[test]
fn ignores_event_store_from_a_newer_schema() {
    let directory = tempdir().expect("tempdir");
    let path = directory.path().join("fleet-events.json");
    fs::write(
        &path,
        format!(
            r#"{{"version":{},"events":[]}}"#,
            PANEFLEET_FLEET_EVENTS_VERSION + 1
        ),
    )
    .expect("write future schema");

    let loaded = PaneFleetFleetEventStore::load_or_default(&path);
    assert_eq!(loaded.version, PANEFLEET_FLEET_EVENTS_VERSION);
    assert_eq!(loaded.recent().count(), 0);
}
