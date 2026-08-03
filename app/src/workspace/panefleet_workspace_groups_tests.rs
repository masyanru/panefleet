use std::collections::HashMap;
use std::path::PathBuf;

use super::group_panefleet_workspaces;
use crate::workspace::panefleet_state::PaneFleetWorkspaceSource;
use crate::workspace::panefleet_tasks::{PaneFleetTaskLabel, PaneFleetTaskState};

fn task(title: &str) -> PaneFleetTaskLabel {
    PaneFleetTaskLabel {
        title: title.to_string(),
        state: PaneFleetTaskState::Working,
    }
}

#[test]
fn groups_primary_and_isolated_worktrees_under_one_repository() {
    let root = PathBuf::from("/projects/sentinel");
    let first = PathBuf::from("/worktrees/sentinel/feature-a");
    let second = PathBuf::from("/worktrees/sentinel/feature-b");
    let sources = HashMap::from([
        (root.clone(), PaneFleetWorkspaceSource::ExistingFolder),
        (
            first.clone(),
            PaneFleetWorkspaceSource::IsolatedWorktree {
                source_repository: root.clone(),
                branch: "feature-a".to_string(),
                managed: true,
            },
        ),
        (
            second.clone(),
            PaneFleetWorkspaceSource::IsolatedWorktree {
                source_repository: root.clone(),
                branch: "feature-b".to_string(),
                managed: true,
            },
        ),
    ]);

    let groups = group_panefleet_workspaces(
        vec![root.clone(), first.clone(), second.clone()],
        &sources,
        &HashMap::new(),
        Some(second),
    );

    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].root_path, root);
    assert_eq!(groups[0].environments.len(), 3);
    assert!(groups[0].environments[0].is_primary);
    assert_eq!(
        groups[0].environments[1].branch.as_deref(),
        Some("feature-a")
    );
    assert_eq!(
        groups[0].environments[2].branch.as_deref(),
        Some("feature-b")
    );
}

#[test]
fn keeps_unrelated_projects_as_separate_groups_in_project_order() {
    let sources = HashMap::new();
    let groups = group_panefleet_workspaces(
        vec![PathBuf::from("/projects/b"), PathBuf::from("/projects/a")],
        &sources,
        &HashMap::new(),
        None,
    );

    assert_eq!(
        groups
            .into_iter()
            .map(|group| group.root_path)
            .collect::<Vec<_>>(),
        vec![PathBuf::from("/projects/b"), PathBuf::from("/projects/a")]
    );
}

#[test]
fn synthesizes_primary_environment_for_a_persisted_isolated_worktree() {
    let root = PathBuf::from("/projects/sentinel");
    let worktree = PathBuf::from("/worktrees/sentinel/feature-a");
    let sources = HashMap::from([(
        worktree.clone(),
        PaneFleetWorkspaceSource::IsolatedWorktree {
            source_repository: root.clone(),
            branch: "feature-a".to_string(),
            managed: true,
        },
    )]);

    let groups = group_panefleet_workspaces(
        vec![root.clone()],
        &sources,
        &HashMap::new(),
        Some(worktree.clone()),
    );

    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].root_path, root);
    assert_eq!(
        groups[0]
            .environments
            .iter()
            .map(|environment| environment.path.clone())
            .collect::<Vec<_>>(),
        vec![PathBuf::from("/projects/sentinel"), worktree]
    );
    assert!(groups[0].environments[0].is_primary);
}

#[test]
fn orders_environments_by_the_task_the_row_shows_not_the_hidden_branch() {
    let root = PathBuf::from("/projects/sentinel");
    let miro = PathBuf::from("/worktrees/sentinel/zz-branch");
    let exposure = PathBuf::from("/worktrees/sentinel/aa-branch");
    let sources = HashMap::from([
        (root.clone(), PaneFleetWorkspaceSource::ExistingFolder),
        (
            miro.clone(),
            PaneFleetWorkspaceSource::IsolatedWorktree {
                source_repository: root.clone(),
                branch: "zz-branch".to_string(),
                managed: true,
            },
        ),
        (
            exposure.clone(),
            PaneFleetWorkspaceSource::IsolatedWorktree {
                source_repository: root.clone(),
                branch: "aa-branch".to_string(),
                managed: true,
            },
        ),
    ]);
    let task_labels = HashMap::from([
        (miro.clone(), task("SEC-1791 · Exposure rule param limit")),
        (exposure.clone(), task("SEC-1802 · Onboard Miro audit logs")),
    ]);

    let groups = group_panefleet_workspaces(
        vec![root.clone(), miro.clone(), exposure.clone()],
        &sources,
        &task_labels,
        None,
    );

    let environments = &groups[0].environments;
    assert!(environments[0].is_primary);
    // Branch order would be aa- then zz-; task order is 1791 then 1802.
    assert_eq!(
        environments[1]
            .task
            .as_ref()
            .map(|task| task.title.as_str()),
        Some("SEC-1791 · Exposure rule param limit")
    );
    assert_eq!(environments[1].path, miro);
    assert_eq!(
        environments[2]
            .task
            .as_ref()
            .map(|task| task.title.as_str()),
        Some("SEC-1802 · Onboard Miro audit logs")
    );
    assert_eq!(environments[2].path, exposure);
}

#[test]
fn a_folder_inside_a_project_becomes_one_of_its_environments() {
    let root = PathBuf::from("/projects/sentinel");
    let cases = PathBuf::from("/projects/sentinel/cases/inc-36884");
    let sources = HashMap::from([
        (root.clone(), PaneFleetWorkspaceSource::ExistingFolder),
        (cases.clone(), PaneFleetWorkspaceSource::ExistingFolder),
    ]);

    let groups = group_panefleet_workspaces(
        vec![root.clone(), cases.clone()],
        &sources,
        &HashMap::new(),
        None,
    );

    // One project, not two: a task's working folder belongs under the project
    // it serves rather than standing beside it.
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].root_path, root);
    assert_eq!(
        groups[0]
            .environments
            .iter()
            .map(|environment| environment.path.clone())
            .collect::<Vec<_>>(),
        vec![root, cases]
    );
}

#[test]
fn a_nested_folder_belongs_to_the_nearest_project() {
    let outer = PathBuf::from("/work");
    let inner = PathBuf::from("/work/sentinel");
    let leaf = PathBuf::from("/work/sentinel/cases/inc-1");
    let sources = HashMap::from([
        (outer.clone(), PaneFleetWorkspaceSource::ExistingFolder),
        (inner.clone(), PaneFleetWorkspaceSource::ExistingFolder),
        (leaf.clone(), PaneFleetWorkspaceSource::ExistingFolder),
    ]);

    let groups = group_panefleet_workspaces(
        vec![outer.clone(), inner.clone(), leaf.clone()],
        &sources,
        &HashMap::new(),
        None,
    );

    let leaf_group = groups
        .iter()
        .find(|group| group.environments.iter().any(|env| env.path == leaf))
        .expect("the leaf is grouped somewhere");
    assert_eq!(
        leaf_group.root_path, inner,
        "nearest project, not outermost"
    );
}

#[test]
fn an_unrelated_project_is_not_swallowed_by_a_path_prefix() {
    // `/projects/sentinel-work` starts with the same characters as
    // `/projects/sentinel` but is not inside it.
    let root = PathBuf::from("/projects/sentinel");
    let sibling = PathBuf::from("/projects/sentinel-work");
    let sources = HashMap::from([
        (root.clone(), PaneFleetWorkspaceSource::ExistingFolder),
        (sibling.clone(), PaneFleetWorkspaceSource::ExistingFolder),
    ]);

    let groups = group_panefleet_workspaces(
        vec![root.clone(), sibling.clone()],
        &sources,
        &HashMap::new(),
        None,
    );

    assert_eq!(groups.len(), 2);
}
