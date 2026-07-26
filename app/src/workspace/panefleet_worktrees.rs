use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use crate::tab_configs::tab_config::generated_worktree_path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PaneFleetCreatedWorktree {
    pub source_repository: PathBuf,
    pub path: PathBuf,
    pub branch: String,
}

pub(super) fn create_panefleet_worktree(
    repository: &Path,
    base_branch: &str,
    requested_branch: Option<&str>,
) -> Result<PaneFleetCreatedWorktree> {
    let repository = git_stdout(repository, &["rev-parse", "--show-toplevel"])
        .context("The selected folder is not a Git repository")
        .map(PathBuf::from)?;
    let base_branch = base_branch.trim();
    if base_branch.is_empty() {
        bail!("Select a base branch");
    }

    let status = git_stdout(&repository, &["status", "--porcelain"])?;
    if !status.trim().is_empty() {
        bail!("Commit or stash changes in the source repository before creating a worktree");
    }

    let existing_branches = crate::util::git::list_local_branches_sync(&repository);
    let branch = requested_branch
        .map(str::trim)
        .filter(|branch| !branch.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| {
            let branch_refs = existing_branches
                .iter()
                .map(String::as_str)
                .collect::<HashSet<_>>();
            warp_util::worktree_names::generate_worktree_branch_name(&branch_refs)
        });

    git_stdout(&repository, &["check-ref-format", "--branch", &branch])
        .with_context(|| format!("'{branch}' is not a valid Git branch name"))?;
    if existing_branches.contains(&branch) {
        bail!("Branch '{branch}' already exists");
    }

    let path = generated_worktree_path(&repository, &branch);
    if path.exists() {
        bail!("Worktree destination already exists: {}", path.display());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create {}", parent.display()))?;
    }

    let path_arg = path.to_string_lossy().into_owned();
    git_stdout(
        &repository,
        &["worktree", "add", "-b", &branch, &path_arg, base_branch],
    )
    .with_context(|| format!("Failed to create worktree for branch '{branch}'"))?;

    Ok(PaneFleetCreatedWorktree {
        source_repository: repository,
        path,
        branch,
    })
}

fn git_stdout(repository: &Path, args: &[&str]) -> Result<String> {
    let output = command::blocking::Command::new("git")
        .args(args)
        .current_dir(repository)
        .stdout(command::Stdio::piped())
        .stderr(command::Stdio::piped())
        .output()
        .with_context(|| format!("Failed to run git {}", args.join(" ")))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let message = stderr.trim();
        if message.is_empty() {
            bail!("git {} failed with {}", args.join(" "), output.status);
        }
        bail!("{message}");
    }
}

#[cfg(test)]
#[path = "panefleet_worktrees_tests.rs"]
mod tests;
