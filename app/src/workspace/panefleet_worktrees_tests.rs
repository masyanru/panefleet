use std::fs;
use std::path::Path;

use super::create_panefleet_worktree;

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
fn refuses_to_create_worktree_from_dirty_repository() {
    let repository = initialized_repository();
    fs::write(repository.path().join("README.md"), "dirty\n").unwrap();

    let error =
        create_panefleet_worktree(repository.path(), "main", Some("feature-dirty")).unwrap_err();

    assert!(
        error
            .to_string()
            .contains("Commit or stash changes in the source repository")
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
