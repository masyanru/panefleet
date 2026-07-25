<p align="center">
  <img src="app/assets/bundled/png/panefleet.png" width="160" alt="PaneFleet app icon" />
</p>

<h1 align="center">PaneFleet</h1>

<p align="center">
  A local-first project and CLI-agent workbench built on the open-source Warp terminal.
</p>

<p align="center">
  <strong>Experimental macOS prototype · Source builds only</strong>
</p>

<!--
Screenshot slot:

<p align="center">
  <img src="docs/images/panefleet-workbench.png" width="1200" alt="PaneFleet workbench" />
</p>
-->

## Why PaneFleet

Terminal tabs are usually global to a window. Agentic development is not.

When several projects and CLI agents are active at once, each project needs its
own working context: terminal tabs, agent conversations, files, Git state, and
session history. Switching projects should switch that entire context, not just
change the current directory.

PaneFleet makes the project workspace the top-level unit:

```text
Project workspace
├── its own horizontal tabs
├── terminal and CLI-agent sessions
├── project files and Git context
└── restored state after an app restart
```

The terminal, editor, file tree, and code-review foundation come from Warp.
PaneFleet adds a project-centric UI and lifecycle for running multiple local CLI
agents side by side.

## What works today

- **Project workspaces** in a persistent left sidebar.
- **Workspace-scoped tabs**: changing projects changes the complete set of open
  terminal and agent tabs.
- **Agent launcher bar** for Terminal, Codex, Claude, OpenCode, and custom agent
  definitions.
- **Configurable CLI agents** with executable, arguments, prompt transport,
  launcher order, and resume adapter settings.
- **Session persistence and resume** for supported CLI agents instead of
  reopening an empty terminal with the old title.
- **Agent activity indicators** that distinguish an active turn from an idle
  CLI process.
- **Project explorer** in the right context sidebar, alongside Changes and
  Review.
- **File actions** for creating files and folders, refreshing the tree, and
  collapsing directories.
- **Git-aware workspace rows** with optional project path and branch metadata.
- **Native Warp terminal and editor capabilities** inside every workspace.

Bundled resume adapters currently cover:

| Agent | New session | Resume |
| --- | --- | --- |
| Codex | `codex` | `codex resume <session-id>` |
| Claude | `claude --session-id <uuid>` | `claude --resume <uuid>` |
| OpenCode | `opencode` | `opencode -s <session-id>` |

Each CLI must be installed and authenticated independently. PaneFleet does not
bundle, proxy, or provide accounts for these services.

## Interface model

```text
Application Window
├── Project Sidebar
│   └── Workspace Rows
├── Workspace Surface
│   ├── Horizontal Tab Strip
│   ├── Agent Launcher Bar
│   └── Active Terminal or Agent Pane
└── Context Sidebar
    ├── Files
    ├── Changes
    └── Review
```

The important rule is that a workspace owns its tabs. Selecting another
workspace replaces the visible tab set; returning restores that workspace's tab
order, active tab, working directories, and resumable agent sessions.

The living UI and session-behavior specification is in
[`specs/panefleet/UI_AND_AGENT_SESSIONS.md`](specs/panefleet/UI_AND_AGENT_SESSIONS.md).

## Privacy and cloud services

PaneFleet is currently local-first:

- Warp telemetry and crash reporting are disabled.
- Warp authentication, cloud agents, session sharing, and settings sync are
  disabled.
- PaneFleet has no account system, subscription service, or cloud sync backend.
- Workspace and agent metadata are stored locally.

The CLI agents and commands launched inside the terminal may still connect to
their own providers or any network destination available to them. Their network
behavior and privacy policies are outside PaneFleet.

If PaneFleet gains optional account or synchronization features later, they
will use a separate PaneFleet service rather than Warp's production endpoints.

## Status and limitations

PaneFleet is an early development prototype, not a finished distribution.

- macOS is the current development and testing platform.
- There are no signed or notarized downloads yet; build from source.
- Windows and Linux support inherited from the upstream codebase has not been
  adapted or verified for the PaneFleet workbench.
- Agent resume depends on the installed CLI version and its locally available
  session history.
- Some inherited Warp implementation details and settings are still being
  separated from the PaneFleet product surface.
- Storage formats and UI behavior may change without migration guarantees while
  the prototype is evolving.

Use it on non-critical workspaces and keep normal Git backups.

## Build on macOS

### Prerequisites

- macOS
- Xcode and its first-launch components
- Rust via [rustup](https://rustup.rs/)
- Git LFS

The repository contains a bootstrap script inherited from Warp. It installs the
full upstream development toolchain, which is larger than PaneFleet itself
currently needs.

```bash
git clone https://github.com/masyanru/panefleet.git
cd panefleet

./script/bootstrap --skip-common-skills --skip-gcloud-auth
cargo run -p warp --bin panefleet
```

If the development dependencies are already installed, the shorter path is:

```bash
git lfs install
git lfs pull
cargo run -p warp --bin panefleet
```

The first build is large because PaneFleet compiles the complete terminal,
editor, and GPU UI stack.

### Build an app bundle

After installing `cargo-bundle`:

```bash
cargo bundle --bin panefleet
open target/debug/bundle/osx/PaneFleet.app
```

The resulting application is locally built and unsigned.

## Development

Useful focused checks:

```bash
cargo fmt --all -- --check
cargo check -p warp --bin panefleet
cargo test -p warp panefleet
```

The full upstream engineering and testing guide remains available in
[`AGENTS.md`](AGENTS.md). PaneFleet-specific product behavior should be updated
in the living specification before or alongside implementation changes.

Issues and focused pull requests are welcome:

- [Report a PaneFleet bug](https://github.com/masyanru/panefleet/issues/new)
- [Browse open issues](https://github.com/masyanru/panefleet/issues)

## Relationship to Warp

PaneFleet is an independent fork of
[`warpdotdev/warp`](https://github.com/warpdotdev/warp). It is not an official
Warp product and is not affiliated with or endorsed by Warp.

The project intentionally reuses Warp's terminal emulator, editor, GPU UI
framework, file explorer, and code-review surfaces while exploring a different
project- and agent-oriented workflow. The Git history and upstream remote are
preserved so changes and attribution remain traceable.

## License

PaneFleet follows the licensing structure of the upstream repository:

- `warpui_core` and `warpui` are available under the
  [MIT License](LICENSE-MIT).
- The rest of the repository is available under the
  [GNU Affero General Public License v3.0](LICENSE-AGPL).

PaneFleet changes follow the license of the files and crates they modify.
Existing copyright and third-party notices remain in their respective files.
