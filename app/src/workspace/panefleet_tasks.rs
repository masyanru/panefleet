//! The intent layer over PaneFleet's mechanism layer.
//!
//! Workspace, environment, tab and agent session all name *how* work happens.
//! A task names *what* the work is. Tasks live in their own versioned state
//! file rather than in `PaneFleetWorkspaceSource`, which was deliberately
//! narrowed and must stay that way.
//!
//! A task is bound to exactly one environment, keyed by the environment's
//! filesystem path — the same identity the rest of PaneFleet addresses
//! workspaces by. Work spanning two repositories is two related tasks, not one
//! task with two environments.

use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub(crate) const PANEFLEET_TASKS_VERSION: u32 = 1;

/// Upper bounds on user-entered text, so a pathological paste cannot bloat the
/// state file or the sidebar row.
const MAX_TITLE_CHARS: usize = 200;
const MAX_EXTERNAL_KEY_CHARS: usize = 32;
/// A tracker key's alphabetic prefix: `SEC`, `inc`, `leaver`. Longer leading
/// words are treated as prose, not as a key.
const MAX_EXTERNAL_KEY_PREFIX_CHARS: usize = 12;

fn current_version() -> u32 {
    PANEFLEET_TASKS_VERSION
}

/// The work axis, deliberately separate from the CLI process axis
/// (`Restoring / InProgress / Blocked / Failed / Success`). "The agent stopped
/// talking" and "the work passed its check" are different claims.
///
/// `Done` is not reachable without passing `done_check`; the gate itself
/// arrives in P2.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PaneFleetTaskState {
    #[default]
    Queued,
    Working,
    NeedsReview,
    AwaitingAck,
    Done,
}

/// A pointer into whatever tracker owns this work. PaneFleet stays
/// vendor-neutral: it stores the key and an optional URL, and never learns
/// what a Jira issue is.
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub(crate) struct PaneFleetTaskExternalRef {
    pub key: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub(crate) struct PaneFleetTaskBinding {
    /// Local identity (`t-0042`), stable across renames. Tracker keys are not
    /// usable as identity: not every task has one.
    pub id: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub external: Option<PaneFleetTaskExternalRef>,
    /// Name of the `TabConfig` this task was created from. Wired up in P1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template: Option<String>,
    #[serde(default)]
    pub state: PaneFleetTaskState,
    /// argv run in the environment's cwd to decide whether the work is
    /// actually done. Consumed in P2; persisted from v1 so the file format
    /// does not have to change when it lands.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub done_check: Option<Vec<String>>,
    /// Ids of tasks that are part of the same effort in other repositories.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub related: Vec<String>,
}

impl PaneFleetTaskBinding {
    pub fn new(id: String, title: String) -> Self {
        Self {
            id,
            title,
            external: None,
            template: None,
            state: PaneFleetTaskState::default(),
            done_check: None,
            related: Vec::new(),
        }
    }

    /// Builds a binding from a single line of user input, splitting off a
    /// leading tracker key so `SEC-1802 Onboard Miro audit logs` yields both an
    /// external reference and a clean title.
    ///
    /// Returns `None` when the input carries no title.
    pub fn from_input(id: String, input: &str) -> Option<Self> {
        let (key, title) = split_external_key(input);
        let title = truncate_chars(title, MAX_TITLE_CHARS);
        if title.is_empty() {
            return None;
        }
        let mut binding = Self::new(id, title);
        binding.external = key.map(|key| PaneFleetTaskExternalRef {
            key,
            provider: None,
            url: None,
        });
        Some(binding)
    }

    /// Rewrites title and external key from user input while preserving id,
    /// state and every field the input cannot express.
    pub fn apply_input(&mut self, input: &str) -> bool {
        let Some(parsed) = Self::from_input(self.id.clone(), input) else {
            return false;
        };
        self.title = parsed.title;
        self.external = parsed.external;
        true
    }

    /// What the sidebar row and the tab show: `SEC-1802 · Onboard Miro audit logs`.
    pub fn label(&self) -> String {
        match &self.external {
            Some(external) if !external.key.is_empty() => {
                format!("{} · {}", external.key, self.title)
            }
            _ => self.title.clone(),
        }
    }

    /// The same text the user typed to create this binding, for pre-filling an
    /// edit field.
    pub fn input_text(&self) -> String {
        match &self.external {
            Some(external) if !external.key.is_empty() => {
                format!("{} {}", external.key, self.title)
            }
            _ => self.title.clone(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct PaneFleetTaskStore {
    #[serde(default = "current_version")]
    pub version: u32,
    /// Keyed by environment path, ordered so the file stays diffable and
    /// readable by other processes.
    #[serde(default)]
    tasks: BTreeMap<PathBuf, PaneFleetTaskBinding>,
    /// How many ids have ever been handed out. Persisted so deleting a task
    /// does not free its id for reuse — a stale entry in another task's
    /// `related` list must never resolve to a different task.
    #[serde(default)]
    issued_id_count: u32,
}

impl Default for PaneFleetTaskStore {
    fn default() -> Self {
        Self {
            version: PANEFLEET_TASKS_VERSION,
            tasks: BTreeMap::new(),
            issued_id_count: 0,
        }
    }
}

impl PaneFleetTaskStore {
    pub fn path() -> PathBuf {
        warp_core::paths::state_dir().join("panefleet-tasks.json")
    }

    /// Reads the store, ignoring a file written by a newer PaneFleet rather
    /// than dropping fields it does not understand.
    pub fn load_or_default(path: &Path) -> Self {
        fs::read(path)
            .ok()
            .and_then(|contents| serde_json::from_slice::<Self>(&contents).ok())
            .filter(|store| store.version <= PANEFLEET_TASKS_VERSION)
            .map(|mut store| {
                store.version = PANEFLEET_TASKS_VERSION;
                store
            })
            .unwrap_or_default()
    }

    pub fn get(&self, environment_path: &Path) -> Option<&PaneFleetTaskBinding> {
        self.tasks.get(environment_path)
    }

    pub fn set(&mut self, environment_path: PathBuf, binding: PaneFleetTaskBinding) {
        self.tasks.insert(environment_path, binding);
    }

    pub fn remove(&mut self, environment_path: &Path) -> Option<PaneFleetTaskBinding> {
        self.tasks.remove(environment_path)
    }

    /// Display labels for every bound environment. Consumers look up by path,
    /// so handing over the whole set cannot miss an environment that the
    /// caller has not registered elsewhere yet.
    pub fn labels(&self) -> HashMap<PathBuf, String> {
        self.tasks
            .iter()
            .map(|(path, task)| (path.clone(), task.label()))
            .collect()
    }

    /// Hands out the next local id. The high-water mark is persisted, so an id
    /// is never reused after its task is deleted; the scan over live tasks only
    /// guards against a hand-edited file whose counter lags behind.
    pub fn allocate_id(&mut self) -> String {
        let highest_live = self
            .tasks
            .values()
            .filter_map(|task| task.id.strip_prefix("t-"))
            .filter_map(|number| number.parse::<u32>().ok())
            .max()
            .unwrap_or(0);
        self.issued_id_count = self.issued_id_count.max(highest_live).saturating_add(1);
        format!("t-{:04}", self.issued_id_count)
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

/// Splits a leading tracker key off a task line.
///
/// A first token counts as a key when it looks like `<word>-<digits>`:
/// `SEC-1802`, `inc-36884`, `PF-14`, `leaver-2026-08`. Anything else is prose
/// and the whole line becomes the title.
fn split_external_key(input: &str) -> (Option<String>, &str) {
    let input = input.trim();
    let Some((candidate, rest)) = input.split_once(char::is_whitespace) else {
        return (None, input);
    };
    let rest = rest.trim_start();
    if rest.is_empty() || !is_external_key(candidate) {
        return (None, input);
    }
    (Some(candidate.to_string()), rest)
}

fn is_external_key(candidate: &str) -> bool {
    if candidate.chars().count() > MAX_EXTERNAL_KEY_CHARS {
        return false;
    }
    let Some((prefix, suffix)) = candidate.split_once('-') else {
        return false;
    };
    let prefix_is_word = !prefix.is_empty()
        && prefix.chars().count() <= MAX_EXTERNAL_KEY_PREFIX_CHARS
        && prefix.starts_with(|character: char| character.is_ascii_alphabetic())
        && prefix
            .chars()
            .all(|character| character.is_ascii_alphanumeric());
    let suffix_is_numeric = !suffix.is_empty()
        && suffix.starts_with(|character: char| character.is_ascii_digit())
        && suffix
            .chars()
            .all(|character| character.is_ascii_digit() || character == '-');
    prefix_is_word && suffix_is_numeric
}

fn truncate_chars(value: &str, maximum_chars: usize) -> String {
    let value = value.trim();
    if value.chars().count() <= maximum_chars {
        return value.to_string();
    }
    let mut truncated = value
        .chars()
        .take(maximum_chars.saturating_sub(1))
        .collect::<String>();
    truncated.push('…');
    truncated
}

#[cfg(test)]
#[path = "panefleet_tasks_tests.rs"]
mod tests;
