use std::fs;
use std::path::Path;

use super::{
    create_panefleet_worktree, inspect_panefleet_worktree_removal, remove_panefleet_worktree,
};

fn git(repository: &Path, args: &[&str]) {
    let status = std::process::Command::new("git")
        .args(args)
        .current_dir(repository)
        .status()
        .unwrap();
    assert!(status.success(), "git {} failed", args.join(" "));
}

fn initialized_repository() -> tempfile::TempDir {
    let repository = tempfile::tempdir().unwrap();
    git(repository.path(), &["init", "-b", "main"]);
    fs::write(repository.path().join("README.md"), "PaneFleet\n").unwrap();
    git(repository.path(), &["add", "README.md"]);
    git(
        repository.path(),
        &[
            "-c",
            "user.name=PaneFleet Tests",
            "-c",
            "user.email=panefleet@example.invalid",
            "commit",
            "-m",
            "initial",
        ],
    );
    repository
}

#[test]
fn creates_isolated_worktree_on_requested_branch() {
    let repository = initialized_repository();
    let branch = format!("feature-isolated-{}", uuid::Uuid::new_v4());

    let worktree =
        create_panefleet_worktree(repository.path(), "main", Some(branch.as_str())).unwrap();

    assert_eq!(worktree.branch, branch);
    assert!(worktree.path.join(".git").is_file());
    assert_eq!(
        fs::read_to_string(worktree.path.join("README.md")).unwrap(),
        "PaneFleet\n"
    );
    git(
        repository.path(),
        &[
            "worktree",
            "remove",
            "--force",
            worktree.path.to_str().unwrap(),
        ],
    );
    git(repository.path(), &["branch", "-D", branch.as_str()]);
}

#[test]
fn creates_a_worktree_from_a_dirty_repository_and_says_so() {
    let repository = initialized_repository();
    fs::write(repository.path().join("README.md"), "dirty\n").unwrap();
    fs::write(repository.path().join("untracked.txt"), "junk\n").unwrap();

    // Refusing here would mean committing or stashing unfinished work just to
    // spin off a parallel environment.
    let worktree =
        create_panefleet_worktree(repository.path(), "main", Some("feature-dirty")).unwrap();

    assert!(worktree.path.exists());
    assert!(worktree.source_had_local_changes);
    // The worktree holds the committed state, so the local changes are not in it.
    assert!(!worktree.path.join("untracked.txt").exists());
    assert_eq!(
        fs::read_to_string(worktree.path.join("README.md")).unwrap(),
        "PaneFleet\n"
    );
    // And they are still in the source, untouched.
    assert_eq!(
        fs::read_to_string(repository.path().join("README.md")).unwrap(),
        "dirty\n"
    );
}

#[test]
fn refuses_to_reuse_existing_branch() {
    let repository = initialized_repository();
    git(repository.path(), &["branch", "feature-existing"]);

    let error =
        create_panefleet_worktree(repository.path(), "main", Some("feature-existing")).unwrap_err();

    assert!(error.to_string().contains("already exists"));
}

#[test]
fn creates_independent_working_directories_for_the_same_repository() {
    let repository = initialized_repository();
    let first_branch = format!("feature-first-{}", uuid::Uuid::new_v4());
    let second_branch = format!("feature-second-{}", uuid::Uuid::new_v4());

    let first =
        create_panefleet_worktree(repository.path(), "main", Some(first_branch.as_str())).unwrap();
    let second =
        create_panefleet_worktree(repository.path(), "main", Some(second_branch.as_str())).unwrap();

    assert_ne!(first.path, second.path);
    fs::write(first.path.join("README.md"), "first workspace\n").unwrap();
    assert_eq!(
        fs::read_to_string(second.path.join("README.md")).unwrap(),
        "PaneFleet\n"
    );

    for worktree in [&first, &second] {
        git(
            repository.path(),
            &[
                "worktree",
                "remove",
                "--force",
                worktree.path.to_str().unwrap(),
            ],
        );
        git(
            repository.path(),
            &["branch", "-D", worktree.branch.as_str()],
        );
    }
}

#[test]
fn removes_clean_worktree_without_deleting_its_branch() {
    let repository = initialized_repository();
    let branch = format!("feature-remove-{}", uuid::Uuid::new_v4());
    let worktree =
        create_panefleet_worktree(repository.path(), "main", Some(branch.as_str())).unwrap();

    let inspection =
        inspect_panefleet_worktree_removal(repository.path(), &worktree.path, &branch).unwrap();
    let outcome = remove_panefleet_worktree(&inspection, false).unwrap();

    assert_eq!(outcome.branch_delete_error, None);
    assert!(!worktree.path.exists());
    git(
        repository.path(),
        &["show-ref", "--verify", &format!("refs/heads/{branch}")],
    );
    git(repository.path(), &["branch", "-D", branch.as_str()]);
}

#[test]
fn refuses_to_remove_dirty_worktree() {
    let repository = initialized_repository();
    let branch = format!("feature-dirty-remove-{}", uuid::Uuid::new_v4());
    let worktree =
        create_panefleet_worktree(repository.path(), "main", Some(branch.as_str())).unwrap();
    fs::write(worktree.path.join("untracked.txt"), "keep me\n").unwrap();

    let error =
        inspect_panefleet_worktree_removal(repository.path(), &worktree.path, &branch).unwrap_err();

    assert!(error.to_string().contains("uncommitted or untracked"));
    assert!(worktree.path.exists());
    git(
        repository.path(),
        &[
            "worktree",
            "remove",
            "--force",
            worktree.path.to_str().unwrap(),
        ],
    );
    git(repository.path(), &["branch", "-D", branch.as_str()]);
}

#[test]
fn safely_deletes_merged_branch_after_removing_worktree() {
    let repository = initialized_repository();
    let branch = format!("feature-remove-branch-{}", uuid::Uuid::new_v4());
    let worktree =
        create_panefleet_worktree(repository.path(), "main", Some(branch.as_str())).unwrap();
    let inspection =
        inspect_panefleet_worktree_removal(repository.path(), &worktree.path, &branch).unwrap();

    let outcome = remove_panefleet_worktree(&inspection, true).unwrap();

    assert_eq!(outcome.branch_delete_error, None);
    assert!(!worktree.path.exists());
    let branch_exists = std::process::Command::new("git")
        .args(["show-ref", "--verify", &format!("refs/heads/{branch}")])
        .current_dir(repository.path())
        .output()
        .unwrap()
        .status
        .success();
    assert!(!branch_exists);
}

#[test]
fn keeps_unmerged_branch_when_safe_deletion_is_requested() {
    let repository = initialized_repository();
    let branch = format!("feature-unmerged-{}", uuid::Uuid::new_v4());
    let worktree =
        create_panefleet_worktree(repository.path(), "main", Some(branch.as_str())).unwrap();
    fs::write(worktree.path.join("feature.txt"), "unmerged\n").unwrap();
    git(&worktree.path, &["add", "feature.txt"]);
    git(
        &worktree.path,
        &[
            "-c",
            "user.name=PaneFleet Tests",
            "-c",
            "user.email=panefleet@example.invalid",
            "commit",
            "-m",
            "unmerged feature",
        ],
    );
    let inspection =
        inspect_panefleet_worktree_removal(repository.path(), &worktree.path, &branch).unwrap();

    let outcome = remove_panefleet_worktree(&inspection, true).unwrap();

    assert!(outcome.branch_delete_error.is_some());
    assert!(!worktree.path.exists());
    git(
        repository.path(),
        &["show-ref", "--verify", &format!("refs/heads/{branch}")],
    );
    git(repository.path(), &["branch", "-D", branch.as_str()]);
}
