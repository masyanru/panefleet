use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use super::panefleet_state::PaneFleetWorkspaceSource;
use super::panefleet_tasks::PaneFleetTaskLabel;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct PaneFleetWorkspaceEnvironment {
    pub path: PathBuf,
    pub branch: Option<String>,
    pub managed: bool,
    pub is_primary: bool,
    /// What this environment is for, e.g. `SEC-1802 · Onboard Miro audit logs`,
    /// with the state of that work. When set it takes the row's first line and
    /// the branch drops to the second.
    pub task: Option<PaneFleetTaskLabel>,
}

impl PaneFleetWorkspaceEnvironment {
    /// The text the row leads with: the task when there is one, otherwise the
    /// branch. Falls back to the empty string so plain folders sort first.
    fn sort_key(&self) -> &str {
        self.task
            .as_ref()
            .map(|task| task.title.as_str())
            .or(self.branch.as_deref())
            .unwrap_or_default()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct PaneFleetWorkspaceGroup {
    pub root_path: PathBuf,
    pub environments: Vec<PaneFleetWorkspaceEnvironment>,
}

pub(super) fn group_panefleet_workspaces(
    ordered_project_paths: Vec<PathBuf>,
    workspace_sources: &HashMap<PathBuf, PaneFleetWorkspaceSource>,
    task_labels: &HashMap<PathBuf, PaneFleetTaskLabel>,
    active_path: Option<PathBuf>,
) -> Vec<PaneFleetWorkspaceGroup> {
    let mut environment_sources = workspace_sources.clone();
    if let Some(active_path) = &active_path {
        environment_sources
            .entry(active_path.clone())
            .or_insert(PaneFleetWorkspaceSource::ExistingFolder);
    }
    for path in &ordered_project_paths {
        environment_sources
            .entry(path.clone())
            .or_insert(PaneFleetWorkspaceSource::ExistingFolder);
    }

    // Every repository a worktree points at, plus every plain folder: the set a
    // nested folder can be a child of.
    let candidate_roots = environment_sources
        .iter()
        .map(|(path, source)| source.project_root(path))
        .collect::<HashSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();

    let mut root_order = Vec::new();
    let mut seen_roots = HashSet::new();
    for path in ordered_project_paths.iter().chain(active_path.iter()) {
        let source = environment_sources.get(path).cloned().unwrap_or_default();
        let root = source_root(path, &source, &candidate_roots);
        if seen_roots.insert(root.clone()) {
            root_order.push(root);
        }
    }

    let mut remaining = environment_sources
        .iter()
        .map(|(path, source)| source_root(path, source, &candidate_roots))
        .filter(|root| !seen_roots.contains(root))
        .collect::<Vec<_>>();
    remaining.sort();
    remaining.dedup();
    root_order.extend(remaining);

    root_order
        .into_iter()
        .map(|root_path| {
            let mut environments = environment_sources
                .iter()
                .filter(|&(path, source)| source_root(path, source, &candidate_roots) == root_path)
                .map(|(path, source)| {
                    environment_for(path, source, &root_path, task_labels.get(path).cloned())
                })
                .collect::<Vec<_>>();
            environments.sort_by(|left, right| {
                right
                    .is_primary
                    .cmp(&left.is_primary)
                    // Order by what the row actually shows, so a list of tasks
                    // does not appear shuffled by hidden branch names.
                    .then_with(|| left.sort_key().cmp(right.sort_key()))
                    .then_with(|| left.path.cmp(&right.path))
            });
            environments.dedup_by(|left, right| left.path == right.path);
            PaneFleetWorkspaceGroup {
                root_path,
                environments,
            }
        })
        .collect()
}

/// The project a path belongs to.
///
/// A worktree names its repository outright. A plain folder is its own project
/// unless it sits **inside** another registered one — a directory environment
/// created for a task lives under the project it serves, and grouping it there
/// is derived from the path rather than stored, so the deliberately narrow
/// `PaneFleetWorkspaceSource` stays as it is.
fn source_root(
    path: &Path,
    source: &PaneFleetWorkspaceSource,
    candidate_roots: &[PathBuf],
) -> PathBuf {
    match source {
        PaneFleetWorkspaceSource::IsolatedWorktree { .. } => source.project_root(path),
        // Longest match wins, so a folder nested two projects deep belongs to
        // the nearer one.
        PaneFleetWorkspaceSource::ExistingFolder => candidate_roots
            .iter()
            .filter(|root| root.as_path() != path && path.starts_with(root))
            .max_by_key(|root| root.components().count())
            .cloned()
            .unwrap_or_else(|| path.to_path_buf()),
    }
}

fn environment_for(
    path: &Path,
    source: &PaneFleetWorkspaceSource,
    root_path: &Path,
    task: Option<PaneFleetTaskLabel>,
) -> PaneFleetWorkspaceEnvironment {
    match source {
        PaneFleetWorkspaceSource::ExistingFolder => PaneFleetWorkspaceEnvironment {
            path: path.to_path_buf(),
            branch: None,
            managed: false,
            is_primary: path == root_path,
            task,
        },
        PaneFleetWorkspaceSource::IsolatedWorktree {
            branch, managed, ..
        } => PaneFleetWorkspaceEnvironment {
            path: path.to_path_buf(),
            branch: Some(branch.clone()),
            managed: *managed,
            is_primary: false,
            task,
        },
    }
}

#[cfg(test)]
#[path = "panefleet_workspace_groups_tests.rs"]
mod tests;
