# PaneFleet development guide

PaneFleet is a local-first macOS workbench for project-scoped terminal and CLI-agent sessions.
It is built on the open-source Warp terminal, so the repository still contains inherited
`warp_*` crate names and cross-platform terminal infrastructure. Product-facing additions
should use PaneFleet naming.

## Common commands

```sh
# Run the GUI
cargo run -p warp --bin panefleet

# Focused validation
cargo fmt --all -- --check
cargo check -p warp --bin panefleet
cargo test -p warp panefleet

# Native dependency setup without optional upstream services
./script/bootstrap --skip-gcloud-auth

# Build a local macOS distribution archive
./script/package-panefleet-macos <version>
```

## Product invariants

- A workspace represents one project directory and owns its horizontal tabs.
- Switching workspaces restores that workspace's active tab set without reordering the
  vertical workspace list.
- New terminals and agents start in the selected workspace directory.
- Supported agents persist their CLI session identifier and resume the actual session rather
  than merely relaunching the executable.
- Activity indicators represent agent work, not just a live CLI process.
- PaneFleet is local-first. Warp telemetry and cloud endpoints remain disabled unless a
  separately designed feature gives users clear controls and documentation.
- The right inspector owns Files, Changes, and Review; the left sidebar owns workspaces only.

The current behavior contract and UI vocabulary live under `specs/panefleet/`.

## Architecture map

- `app/` — desktop application and product surfaces.
- `app/src/panefleet/` — PaneFleet workspace, agent, persistence, and UI integration.
- `crates/warp_core/` — shared models and platform abstractions.
- `crates/warpui/` and `crates/warpui_core/` — custom UI and rendering framework.
- `crates/terminal/` and related crates — terminal emulation and shell integration.
- `crates/warp_tui/` — inherited headless TUI frontend.
- `resources/` — app icons, themes, shell resources, and bundled assets.
- `script/` — bootstrap, run, verification, and packaging helpers.

## Coding guidelines

- Follow the established Rust style and keep `ctx` as the final context parameter.
- Prefer exhaustive matching over wildcard arms when variants are expected to evolve.
- Keep `TerminalModel` lock scopes short. Do not acquire a second lock through a nested call;
  this can freeze the UI.
- Preserve comments that still describe the code; avoid unrelated mechanical rewrites.
- Never log prompts, session contents, credentials, or other confidential terminal data.
- Add unit tests in a neighboring `*_tests.rs` or `mod_test.rs` file when practical.
- For GUI work, follow `.agents/skills/gui-ui-guidelines/SKILL.md`.
- For Rust tests, follow `.agents/skills/rust-unit-tests/SKILL.md`.

## UI changes

Verify visual changes in a real PaneFleet window. Check at minimum:

- switching between two workspaces with different tab sets;
- closing and restoring agent tabs;
- narrow and wide workspace/inspector sidebars;
- an empty or non-Git workspace;
- keyboard focus and terminal input after every navigation action.

Include screenshots in pull requests when layout or styling changes.

## Upstream changes

Keep `upstream` pointed at `warpdotdev/warp`. Integrate upstream through a dedicated branch,
preserve upstream commits, and isolate PaneFleet-specific conflict resolutions. Do not remove
apparently unused terminal or platform code solely because the current alpha ships for macOS;
it may still be required by Cargo features or future upstream merges.

## Security and licensing

Do not publish suspected vulnerabilities; follow `SECURITY.md`. Preserve the repository's
AGPL and MIT notices and the attribution required by inherited Warp code.
