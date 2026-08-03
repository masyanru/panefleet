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
/// Folder names stay short enough to read in a sidebar row.
const MAX_SLUG_CHARS: usize = 48;

fn current_version() -> u32 {
    PANEFLEET_TASKS_VERSION
}

/// The work axis, deliberately separate from the CLI process axis
/// (`Restoring / InProgress / Blocked / Failed / Success`). "The agent stopped
/// talking" and "the work passed its check" are different claims.
///
/// `Done` is only ever set by a person. A passing gate yields `AwaitingAck` —
/// "the check passed, look at it" — because in practice a task that passed once
/// usually takes more prompts to refine, so a passing check is a current
/// property rather than an achievement. Marking it done is the human step.
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

impl PaneFleetTaskState {
    /// Label for surfaces that show the work axis beside the mechanism.
    pub fn label(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Working => "working",
            Self::NeedsReview => "needs review",
            Self::AwaitingAck => "awaiting ack",
            Self::Done => "done",
        }
    }
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
    /// actually done.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub done_check: Option<Vec<String>>,
    /// When a person marked this done, as unix milliseconds. Lets a surface ask
    /// "how many in the last day" — impossible while `Done` was just a state
    /// with no moment attached.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed_at_unix_ms: Option<u64>,
    /// Set when the task was marked done without a passing gate behind it — no
    /// gate configured, or the last run failed. The confirmation then rests on
    /// the person's word, and a card should not present it as checked.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub completed_without_gate: bool,
    /// Verdict of the most recent gate run, independent of the current state.
    ///
    /// The state is not a usable substitute: passing the gate and then sending
    /// another prompt moves the task back to `Working`, and confirming from
    /// there would otherwise look like confirming with no check at all.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_gate_passed: Option<bool>,
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
            completed_at_unix_ms: None,
            completed_without_gate: false,
            last_gate_passed: None,
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

    /// The gate as a shell command line, for showing and editing it.
    pub fn done_check_text(&self) -> String {
        self.done_check
            .as_ref()
            .map(|argv| shell_words::join(argv.iter().map(String::as_str)))
            .unwrap_or_default()
    }

    /// Parses a gate from a shell command line. An unparseable line leaves the
    /// existing gate alone rather than silently dropping it.
    pub fn set_done_check_from_text(&mut self, text: &str) -> bool {
        let text = text.trim();
        if text.is_empty() {
            self.done_check = None;
            return true;
        }
        match shell_words::split(text) {
            Ok(argv) if !argv.is_empty() => {
                self.done_check = Some(argv);
                true
            }
            _ => false,
        }
    }

    /// Records the outcome of a gate run. Zero means the work passed.
    ///
    /// A pass yields `AwaitingAck`, not `Done`: the check passing is evidence
    /// for a person, not a substitute for them.
    pub fn apply_done_check_outcome(&mut self, passed: bool) {
        self.state = if passed {
            PaneFleetTaskState::AwaitingAck
        } else {
            PaneFleetTaskState::NeedsReview
        };
        self.completed_at_unix_ms = None;
        self.completed_without_gate = false;
        self.last_gate_passed = Some(passed);
    }

    /// Marks the work finished. The only route to `Done`.
    ///
    /// Records whether a passing gate stood behind the decision, so a surface
    /// can tell "checked and confirmed" from "confirmed on someone's word".
    pub fn mark_done(&mut self, now_unix_ms: u64) {
        self.completed_without_gate = self.last_gate_passed != Some(true);
        self.state = PaneFleetTaskState::Done;
        self.completed_at_unix_ms = Some(now_unix_ms);
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

    /// A folder name for a directory environment serving this task.
    ///
    /// Prefers the tracker key — short, stable across renames, and what the
    /// person already uses to refer to the work.
    pub fn directory_slug(&self) -> String {
        let source = match &self.external {
            Some(external) if !external.key.is_empty() => external.key.as_str(),
            _ => self.title.as_str(),
        };
        let mut slug = String::new();
        for character in source.chars() {
            if character.is_ascii_alphanumeric() {
                slug.push(character.to_ascii_lowercase());
            } else if !slug.ends_with('-') {
                slug.push('-');
            }
        }
        let slug = slug.trim_matches('-');
        let slug = if slug.is_empty() { "task" } else { slug };
        slug.chars().take(MAX_SLUG_CHARS).collect()
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

/// What a surface needs to render a bound task: the work axis alongside the
/// name, so "needs review" is visible without opening anything.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PaneFleetTaskLabel {
    pub title: String,
    pub state: PaneFleetTaskState,
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
    /// Set when `load_or_default` found a file it could not use — corrupt, or
    /// written by a newer build. Writing then would replace real bindings with
    /// this empty store, turning a read failure into permanent data loss.
    #[serde(skip)]
    unwritable: bool,
}

impl Default for PaneFleetTaskStore {
    fn default() -> Self {
        Self {
            version: PANEFLEET_TASKS_VERSION,
            tasks: BTreeMap::new(),
            issued_id_count: 0,
            unwritable: false,
        }
    }
}

impl PaneFleetTaskStore {
    pub fn path() -> PathBuf {
        warp_core::paths::state_dir().join("panefleet-tasks.json")
    }

    /// Reads the store, ignoring a file written by a newer PaneFleet rather
    /// than dropping fields it does not understand.
    ///
    /// A file that exists but cannot be used leaves the store `unwritable`, so
    /// the next edit does not overwrite bindings this build simply failed to
    /// read.
    pub fn load_or_default(path: &Path) -> Self {
        let Ok(contents) = fs::read(path) else {
            return Self::default();
        };
        let usable = serde_json::from_slice::<Self>(&contents)
            .ok()
            .filter(|store| store.version <= PANEFLEET_TASKS_VERSION);
        match usable {
            Some(mut store) => {
                store.version = PANEFLEET_TASKS_VERSION;
                store
            }
            None => Self {
                unwritable: true,
                ..Self::default()
            },
        }
    }

    pub fn get(&self, environment_path: &Path) -> Option<&PaneFleetTaskBinding> {
        self.tasks.get(environment_path)
    }

    pub fn set(&mut self, environment_path: PathBuf, binding: PaneFleetTaskBinding) {
        // Raise the high-water mark here rather than only in `allocate_id`, so
        // removing a binding can never lower it below an id that was once in
        // use — whatever route the binding came in by.
        if let Some(number) = binding
            .id
            .strip_prefix("t-")
            .and_then(|number| number.parse::<u32>().ok())
        {
            self.issued_id_count = self.issued_id_count.max(number);
        }
        self.tasks.insert(environment_path, binding);
    }

    pub fn remove(&mut self, environment_path: &Path) -> Option<PaneFleetTaskBinding> {
        self.tasks.remove(environment_path)
    }

    /// Drops bindings whose environment directory no longer exists.
    ///
    /// A worktree removed with `git worktree remove`, or a folder deleted
    /// outside the app, never reaches the removal flow. Without this the file
    /// only grows, and a path later reused by a new environment silently
    /// inherits the old task's title and work state.
    ///
    /// Returns whether anything was dropped.
    pub fn prune_missing_environments(&mut self) -> bool {
        let before = self.tasks.len();
        self.tasks.retain(|path, _| path.exists());
        self.tasks.len() != before
    }

    /// Every binding, for surfaces that need the whole set rather than only the
    /// environments that happen to be open.
    pub fn entries(&self) -> impl Iterator<Item = (&PathBuf, &PaneFleetTaskBinding)> {
        self.tasks.iter()
    }

    /// Display labels for every bound environment. Consumers look up by path,
    /// so handing over the whole set cannot miss an environment that the
    /// caller has not registered elsewhere yet.
    pub fn labels(&self) -> HashMap<PathBuf, PaneFleetTaskLabel> {
        self.tasks
            .iter()
            .map(|(path, task)| {
                (
                    path.clone(),
                    PaneFleetTaskLabel {
                        title: task.label(),
                        state: task.state,
                    },
                )
            })
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
        if self.unwritable {
            return Err(io::Error::other(
                "refusing to overwrite a task file this build could not read",
            ));
        }
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
