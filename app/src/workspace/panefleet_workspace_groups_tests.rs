use std::collections::HashMap;
use std::path::PathBuf;

use super::group_panefleet_workspaces;
use crate::workspace::panefleet_state::PaneFleetWorkspaceSource;

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
        (
            miro.clone(),
            "SEC-1791 · Exposure rule param limit".to_string(),
        ),
        (
            exposure.clone(),
            "SEC-1802 · Onboard Miro audit logs".to_string(),
        ),
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
        environments[1].task_label.as_deref(),
        Some("SEC-1791 · Exposure rule param limit")
    );
    assert_eq!(environments[1].path, miro);
    assert_eq!(
        environments[2].task_label.as_deref(),
        Some("SEC-1802 · Onboard Miro audit logs")
    );
    assert_eq!(environments[2].path, exposure);
}
