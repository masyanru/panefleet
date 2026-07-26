use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use super::panefleet_state::PaneFleetWorkspaceSource;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct PaneFleetWorkspaceEnvironment {
    pub path: PathBuf,
    pub branch: Option<String>,
    pub managed: bool,
    pub is_primary: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct PaneFleetWorkspaceGroup {
    pub root_path: PathBuf,
    pub environments: Vec<PaneFleetWorkspaceEnvironment>,
}

pub(super) fn group_panefleet_workspaces(
    ordered_project_paths: Vec<PathBuf>,
    workspace_sources: &HashMap<PathBuf, PaneFleetWorkspaceSource>,
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

    let mut root_order = Vec::new();
    let mut seen_roots = HashSet::new();
    for path in ordered_project_paths.iter().chain(active_path.iter()) {
        let source = environment_sources.get(path).cloned().unwrap_or_default();
        let root = source_root(path, &source);
        if seen_roots.insert(root.clone()) {
            root_order.push(root);
        }
    }

    let mut remaining = environment_sources
        .iter()
        .map(|(path, source)| source_root(path, source))
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
                .filter_map(|(path, source)| {
                    (source_root(path, source) == root_path)
                        .then(|| environment_for(path, source, &root_path))
                })
                .collect::<Vec<_>>();
            environments.sort_by(|left, right| {
                right
                    .is_primary
                    .cmp(&left.is_primary)
                    .then_with(|| left.branch.cmp(&right.branch))
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

fn source_root(path: &Path, source: &PaneFleetWorkspaceSource) -> PathBuf {
    source.project_root(path)
}

fn environment_for(
    path: &Path,
    source: &PaneFleetWorkspaceSource,
    root_path: &Path,
) -> PaneFleetWorkspaceEnvironment {
    match source {
        PaneFleetWorkspaceSource::ExistingFolder => PaneFleetWorkspaceEnvironment {
            path: path.to_path_buf(),
            branch: None,
            managed: false,
            is_primary: path == root_path,
        },
        PaneFleetWorkspaceSource::IsolatedWorktree {
            branch, managed, ..
        } => PaneFleetWorkspaceEnvironment {
            path: path.to_path_buf(),
            branch: Some(branch.clone()),
            managed: *managed,
            is_primary: false,
        },
    }
}

#[cfg(test)]
#[path = "panefleet_workspace_groups_tests.rs"]
mod tests;
