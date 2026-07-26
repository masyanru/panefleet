use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use warpui::EntityId;

use crate::terminal::CLIAgent;

pub(crate) const PANEFLEET_FLEET_EVENTS_VERSION: u32 = 1;
const MAX_FLEET_EVENTS: usize = 500;

fn current_version() -> u32 {
    PANEFLEET_FLEET_EVENTS_VERSION
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PaneFleetFleetEventKind {
    Started,
    NeedsInput,
    Completed,
    Failed,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct PaneFleetFleetEvent {
    pub id: Uuid,
    pub occurred_at_unix_ms: u64,
    pub workspace_path: PathBuf,
    pub agent: CLIAgent,
    pub kind: PaneFleetFleetEventKind,
    /// Runtime-only navigation target. Entity IDs are process-local and must never be persisted.
    #[serde(skip)]
    pub terminal_view_id: Option<EntityId>,
}

impl PaneFleetFleetEvent {
    pub fn new(
        workspace_path: PathBuf,
        agent: CLIAgent,
        kind: PaneFleetFleetEventKind,
        terminal_view_id: EntityId,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            occurred_at_unix_ms: unix_time_ms(),
            workspace_path,
            agent,
            kind,
            terminal_view_id: Some(terminal_view_id),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct PaneFleetFleetEventStore {
    #[serde(default = "current_version")]
    pub version: u32,
    #[serde(default)]
    events: Vec<PaneFleetFleetEvent>,
}

impl Default for PaneFleetFleetEventStore {
    fn default() -> Self {
        Self {
            version: PANEFLEET_FLEET_EVENTS_VERSION,
            events: Vec::new(),
        }
    }
}

impl PaneFleetFleetEventStore {
    pub fn path() -> PathBuf {
        warp_core::paths::state_dir().join("panefleet-fleet-events.json")
    }

    pub fn load_or_default(path: &Path) -> Self {
        fs::read(path)
            .ok()
            .and_then(|contents| serde_json::from_slice::<Self>(&contents).ok())
            .filter(|store| store.version <= PANEFLEET_FLEET_EVENTS_VERSION)
            .map(|mut store| {
                store.events.truncate_from_start(MAX_FLEET_EVENTS);
                store
            })
            .unwrap_or_default()
    }

    pub fn record(&mut self, event: PaneFleetFleetEvent) {
        self.events.push(event);
        self.events.truncate_from_start(MAX_FLEET_EVENTS);
    }

    pub fn recent(&self) -> impl Iterator<Item = &PaneFleetFleetEvent> {
        self.events.iter().rev()
    }

    pub fn clear_terminal_target(&mut self, terminal_view_id: EntityId) {
        for event in &mut self.events {
            if event.terminal_view_id == Some(terminal_view_id) {
                event.terminal_view_id = None;
            }
        }
    }

    pub fn write_atomic(&self, path: &Path) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let temporary_path = path.with_extension("json.tmp");
        let contents = serde_json::to_vec_pretty(self).map_err(io::Error::other)?;
        fs::write(&temporary_path, contents)?;
        match fs::rename(&temporary_path, path) {
            Ok(()) => Ok(()),
            Err(error) => {
                let _ = fs::remove_file(&temporary_path);
                Err(error)
            }
        }
    }
}

fn unix_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

trait TruncateFromStart {
    fn truncate_from_start(&mut self, maximum_len: usize);
}

impl<T> TruncateFromStart for Vec<T> {
    fn truncate_from_start(&mut self, maximum_len: usize) {
        if self.len() > maximum_len {
            self.drain(..self.len() - maximum_len);
        }
    }
}

#[cfg(test)]
#[path = "panefleet_fleet_events_tests.rs"]
mod tests;
