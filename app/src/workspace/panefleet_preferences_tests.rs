use std::fs;

use super::{
    PANEFLEET_WORKSPACE_PREFERENCES_VERSION, PaneFleetWorkspaceIcon, PaneFleetWorkspacePreferences,
    git_config_references_github, parse_git_head, workspace_icon_for_path,
};

#[test]
fn defaults_all_workspace_indicators_to_visible() {
    assert_eq!(
        PaneFleetWorkspacePreferences::default(),
        PaneFleetWorkspacePreferences {
            version: PANEFLEET_WORKSPACE_PREFERENCES_VERSION,
            show_workspace_path: true,
            show_git_branch: true,
            show_agent_activity: true,
        }
    );
}

#[test]
fn missing_fields_in_older_preferences_receive_defaults() {
    let preferences = PaneFleetWorkspacePreferences::decode(br#"{"version": 1}"#).unwrap();

    assert!(preferences.show_workspace_path);
    assert!(preferences.show_git_branch);
    assert!(preferences.show_agent_activity);
}

#[test]
fn parses_branch_and_detached_head_for_sidebar_display() {
    assert_eq!(
        parse_git_head("ref: refs/heads/feature/panefleet\n"),
        Some("feature/panefleet".to_string())
    );
    assert_eq!(
        parse_git_head("1234567890abcdef\n"),
        Some("12345678".to_string())
    );
}

#[test]
fn identifies_github_and_other_git_workspaces() {
    let root = tempfile::tempdir().unwrap();
    let github_workspace = root.path().join("github");
    let git_workspace = root.path().join("gitlab");
    let folder_workspace = root.path().join("folder");

    for workspace in [&github_workspace, &git_workspace] {
        fs::create_dir_all(workspace.join(".git")).unwrap();
    }
    fs::create_dir_all(&folder_workspace).unwrap();
    fs::write(
        github_workspace.join(".git/config"),
        "[remote \"origin\"]\n\turl = git@github.com:masyanru/panefleet.git\n",
    )
    .unwrap();
    fs::write(
        git_workspace.join(".git/config"),
        "[remote \"origin\"]\n\turl = https://gitlab.example.com/team/project.git\n",
    )
    .unwrap();

    assert_eq!(
        workspace_icon_for_path(&github_workspace),
        PaneFleetWorkspaceIcon::Github
    );
    assert_eq!(
        workspace_icon_for_path(&git_workspace),
        PaneFleetWorkspaceIcon::Git
    );
    assert_eq!(
        workspace_icon_for_path(&folder_workspace),
        PaneFleetWorkspaceIcon::PaneFleet
    );
}

#[test]
fn only_treats_remote_urls_as_github_repositories() {
    assert!(git_config_references_github(
        "[remote \"origin\"]\nurl = https://github.com/masyanru/panefleet.git"
    ));
    assert!(!git_config_references_github(
        "[alias]\nexample = echo https://github.com/not-a-remote"
    ));
}
