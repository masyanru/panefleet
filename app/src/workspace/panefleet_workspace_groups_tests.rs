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

    let groups = group_panefleet_workspaces(vec![root.clone()], &sources, Some(worktree.clone()));

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
