# Contributing to PaneFleet

Thanks for helping improve PaneFleet. The project is an experimental macOS workbench for
project-scoped terminal and CLI-agent sessions, built as a downstream fork of Warp.

## Before you start

- Search the [existing issues](https://github.com/masyanru/panefleet/issues).
- Bug fixes and focused improvements are welcome.
- For larger UI, persistence, or session-lifecycle changes, open an issue first so the
  expected behavior can be agreed before implementation.
- If a bug reproduces in an unmodified Warp build and is not PaneFleet-specific, consider
  reporting it to the [upstream Warp repository](https://github.com/warpdotdev/warp/issues).

## Development setup

PaneFleet currently targets macOS. You need Xcode command-line tools, Git LFS, and the Rust
toolchain pinned by `rust-toolchain.toml`.

```sh
git lfs install
./script/bootstrap --skip-gcloud-auth
cargo run -p warp --bin panefleet
```

The bootstrap script is inherited from Warp and installs native build dependencies. The
`--skip-gcloud-auth` flag avoids an optional upstream developer service.

## Project principles

- Workspace state is project-scoped: switching projects must switch the associated tabs and
  sessions without reordering the project list.
- Agent processes start in the workspace directory and should resume their real CLI session
  when a supported session identifier is available.
- PaneFleet is local-first. Do not add telemetry or cloud calls without an explicit product
  decision, visible user controls, and documentation.
- Preserve compatibility with the inherited terminal, editor, and rendering core where
  practical; this keeps upstream updates manageable.
- Never commit credentials, local session data, signing identities, or generated release
  archives.

## Validation

Run the checks that match your change:

```sh
cargo fmt --all -- --check
cargo check -p warp --bin panefleet
cargo test -p warp panefleet
```

For visual changes, also launch the app and include a screenshot or short recording. For
packaging changes, validate the scripts without publishing an artifact:

```sh
bash -n script/generate-panefleet-macos-icon
bash -n script/package-panefleet-macos
```

## Pull requests

- Keep each PR focused on one logical change.
- Explain the user-visible behavior and how it was verified.
- Link the related issue when one exists.
- Include screenshots for UI changes.
- Call out inherited Warp code that was modified, especially changes likely to conflict with
  future upstream merges.

By contributing, you agree that your contribution is provided under the repository's
applicable [AGPL-3.0](LICENSE-AGPL) and [MIT](LICENSE-MIT) licenses.

## Upstream synchronization

`upstream` should point to `https://github.com/warpdotdev/warp.git`. Merge upstream changes in
a dedicated branch, resolve PaneFleet product conflicts deliberately, and keep unrelated
upstream commits intact rather than rewriting their history.
