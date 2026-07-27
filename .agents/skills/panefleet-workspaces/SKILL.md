---
name: panefleet-workspaces
description: Implement and review PaneFleet project workspaces, grouped Git worktree environments, workspace-scoped tabs, Fleet Dashboard state, and their persistence and lifecycle. Use for changes to the left workspace sidebar, workspace switching, worktree creation/removal, Fleet workspace relationships, last-tab behavior, or files under app/src/workspace/panefleet_* and the related portions of workspace/view.rs.
---

# PaneFleet Workspaces

Treat a project as the durable parent and Git worktrees as related environments
inside it. Preserve tabs, agent sessions, paths, Git state, and Fleet identity
across switching and restart.

## Start with the contract

1. Read `AGENTS.md`.
2. Read `specs/panefleet/UI_AND_AGENT_SESSIONS.md`.
3. For UI changes, also read `.agents/skills/gui-ui-guidelines/SKILL.md`.
4. Inspect the persisted model and tests before changing rendering.

Key modules:

- `app/src/workspace/panefleet_state.rs`: persisted tabs and project state.
- `app/src/workspace/panefleet_workspace_groups.rs`: project/worktree relationships.
- `app/src/workspace/panefleet_worktrees.rs`: Git lifecycle and safety checks.
- `app/src/workspace/view/left_panel.rs`: workspace rows and pointer events.
- `app/src/workspace/view.rs`: orchestration, actions, overlays, and Fleet rendering.
- Neighboring `*_tests.rs` files: expected behavior and migration coverage.

## Preserve product invariants

- Keep the left project order stable when switching.
- Group each managed worktree under its source project in both the sidebar and
  Fleet Dashboard.
- Make every environment own its tabs, selected tab, agent processes, working
  directory, and resumable agent identities.
- Start terminals and agents in the selected environment path.
- Never synthesize a persistent Home-directory workspace merely because the
  last tab or global Fleet view closes.
- Keep global Settings and Fleet views separate from a project's launcher bar.
- Treat activity as actual agent work, not merely a running CLI process.
- Keep external linked folders unmanaged: PaneFleet may forget them but must
  not physically delete them.

## Implement worktree lifecycle safely

For creation:

- Resolve the source repository with Git.
- Require a clean source working tree.
- Validate the base branch and requested branch.
- Use `git worktree add`; do not copy the repository.
- Persist source repository, environment path, branch, and managed state.

For removal:

- Canonicalize the path and verify it appears in `git worktree list --porcelain`.
- Verify the expected branch and reject detached HEAD.
- Block modified and untracked files.
- Re-run the inspection immediately before removal.
- Close tabs and agent processes, then use `git worktree remove`; never use
  recursive filesystem deletion.
- Keep the branch by default.
- When explicitly requested, use only `git branch -d`. Never silently fall
  back to `git branch -D`; retain unmerged branches and show a concise result.

## Coordinate UI state

- Route row actions through typed `LeftPanelEvent` and `WorkspaceAction`
  variants instead of mutating workspace state inside row rendering.
- Reuse existing `Menu`, `Dialog`, action-button themes, and overlay placement.
- When a menu item opens a modal, defer the modal transition to the next UI
  update if the menu still has close/select actions queued.
- Notify both a child view after changing its render source and the owning
  workspace after changing overlay visibility.
- On failure after an environment was closed, restore a usable environment
  rather than leaving persisted state half-removed.
- Put detailed Git stderr in logs and show a short actionable message in UI.

## Validate

Run:

```sh
cargo fmt --all -- --check
cargo check -p warp --bin panefleet
cargo test -p warp panefleet
```

Add focused tests for relationship restoration and destructive Git operations.
Then verify in a real PaneFleet window: switching parent projects, selecting
multiple worktrees under one project, closing the last tab, opening Fleet,
canceling removal, rejecting a dirty worktree, and removing a clean worktree.
