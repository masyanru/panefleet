use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context as _, Result, anyhow};
use semver::Version;
use serde::{Deserialize, Serialize};
use warpui::r#async::Timer;
use warpui::{Entity, ModelContext, SingletonEntity};

use crate::channel::ChannelState;
use crate::features::FeatureFlag;

const RELEASES_URL: &str = "https://api.github.com/repos/masyanru/panefleet/releases?per_page=20";
const INITIAL_CHECK_DELAY: Duration = Duration::from_secs(3);
const CHECK_INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);
const STATE_VERSION: u32 = 1;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PaneFleetRelease {
    pub tag_name: String,
    pub html_url: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) enum PaneFleetUpdateStatus {
    #[default]
    Idle,
    Checking,
    UpToDate,
    Available(PaneFleetRelease),
    Error(String),
}

pub(crate) enum PaneFleetUpdateEvent {
    Changed,
}

#[derive(Debug, Deserialize, Serialize)]
struct PersistedUpdateState {
    #[serde(default = "state_version")]
    version: u32,
    #[serde(default)]
    dismissed_tag: Option<String>,
}

fn state_version() -> u32 {
    STATE_VERSION
}

impl Default for PersistedUpdateState {
    fn default() -> Self {
        Self {
            version: STATE_VERSION,
            dismissed_tag: None,
        }
    }
}

impl PersistedUpdateState {
    fn path() -> PathBuf {
        warp_core::paths::state_dir().join("panefleet-update-state.json")
    }

    fn load(path: &Path) -> Self {
        fs::read(path)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .filter(|state: &Self| state.version <= STATE_VERSION)
            .unwrap_or_default()
    }

    fn write_atomic(&self, path: &Path) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let temporary_path = path.with_extension("json.tmp");
        let contents = serde_json::to_vec_pretty(self).map_err(io::Error::other)?;
        fs::write(&temporary_path, contents)?;
        match fs::rename(&temporary_path, path) {
            Ok(()) => Ok(()),
            Err(error) => {
                let _ = fs::remove_file(&temporary_path);
                Err(error)
            }
        }
    }
}

pub(crate) struct PaneFleetUpdateChecker {
    client: Arc<http_client::Client>,
    status: PaneFleetUpdateStatus,
    persisted: PersistedUpdateState,
    state_path: PathBuf,
    timer_scheduled: bool,
}

impl PaneFleetUpdateChecker {
    pub fn new(ctx: &mut ModelContext<Self>) -> Self {
        let state_path = PersistedUpdateState::path();
        let persisted = PersistedUpdateState::load(&state_path);
        let mut checker = Self {
            client: Arc::new(http_client::Client::new()),
            status: PaneFleetUpdateStatus::Idle,
            persisted,
            state_path,
            timer_scheduled: false,
        };
        if FeatureFlag::PaneFleetWorkbench.is_enabled() {
            checker.schedule_initial_check(ctx);
        }
        checker
    }

    #[cfg(test)]
    pub fn new_for_test(_ctx: &mut ModelContext<Self>) -> Self {
        Self {
            client: Arc::new(http_client::Client::new()),
            status: PaneFleetUpdateStatus::Idle,
            persisted: PersistedUpdateState::default(),
            state_path: PathBuf::new(),
            timer_scheduled: false,
        }
    }

    pub fn status(&self) -> &PaneFleetUpdateStatus {
        &self.status
    }

    pub fn current_version_label() -> &'static str {
        ChannelState::app_version().unwrap_or("development build")
    }

    pub fn available_release_for_banner(&self) -> Option<&PaneFleetRelease> {
        let PaneFleetUpdateStatus::Available(release) = &self.status else {
            return None;
        };
        (self.persisted.dismissed_tag.as_deref() != Some(release.tag_name.as_str()))
            .then_some(release)
    }

    pub fn manually_check_for_update(&mut self, ctx: &mut ModelContext<Self>) {
        self.check_for_update(ctx);
    }

    pub fn dismiss_available_update(&mut self, ctx: &mut ModelContext<Self>) {
        let PaneFleetUpdateStatus::Available(release) = &self.status else {
            return;
        };
        self.persisted.dismissed_tag = Some(release.tag_name.clone());
        if let Err(error) = self.persisted.write_atomic(&self.state_path) {
            log::warn!("Failed to persist dismissed PaneFleet update: {error}");
        }
        ctx.emit(PaneFleetUpdateEvent::Changed);
        ctx.notify();
    }

    fn schedule_initial_check(&mut self, ctx: &mut ModelContext<Self>) {
        if self.timer_scheduled {
            return;
        }
        self.timer_scheduled = true;
        ctx.spawn(
            async {
                Timer::after(INITIAL_CHECK_DELAY).await;
            },
            |checker, _, ctx| {
                checker.timer_scheduled = false;
                checker.check_for_update(ctx);
            },
        );
    }

    fn schedule_next_check(&mut self, ctx: &mut ModelContext<Self>) {
        if self.timer_scheduled {
            return;
        }
        self.timer_scheduled = true;
        ctx.spawn(
            async {
                Timer::after(CHECK_INTERVAL).await;
            },
            |checker, _, ctx| {
                checker.timer_scheduled = false;
                checker.check_for_update(ctx);
            },
        );
    }

    fn check_for_update(&mut self, ctx: &mut ModelContext<Self>) {
        if matches!(self.status, PaneFleetUpdateStatus::Checking) {
            return;
        }

        self.status = PaneFleetUpdateStatus::Checking;
        ctx.emit(PaneFleetUpdateEvent::Changed);
        ctx.notify();

        let client = self.client.clone();
        let current_tag = ChannelState::app_version().map(str::to_owned);
        ctx.spawn(
            async move { fetch_available_release(&client, current_tag.as_deref()).await },
            |checker, result, ctx| {
                checker.status = match result {
                    Ok(Some(release)) => {
                        log::info!("PaneFleet update available: {}", release.tag_name);
                        PaneFleetUpdateStatus::Available(release)
                    }
                    Ok(None) => {
                        log::info!("PaneFleet is up to date");
                        PaneFleetUpdateStatus::UpToDate
                    }
                    Err(error) => {
                        log::warn!("PaneFleet update check failed: {error:#}");
                        PaneFleetUpdateStatus::Error(error.to_string())
                    }
                };
                ctx.emit(PaneFleetUpdateEvent::Changed);
                ctx.notify();
                checker.schedule_next_check(ctx);
            },
        );
    }
}

impl Entity for PaneFleetUpdateChecker {
    type Event = PaneFleetUpdateEvent;
}

impl SingletonEntity for PaneFleetUpdateChecker {}

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    html_url: String,
    draft: bool,
    assets: Vec<GitHubAsset>,
}

#[derive(Debug, Deserialize)]
struct GitHubAsset {
    name: String,
}

async fn fetch_available_release(
    client: &http_client::Client,
    current_tag: Option<&str>,
) -> Result<Option<PaneFleetRelease>> {
    let user_agent = format!(
        "PaneFleet/{}",
        current_tag.unwrap_or(env!("CARGO_PKG_VERSION"))
    );
    let response = client
        .get(RELEASES_URL)
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .header("User-Agent", user_agent)
        .timeout(REQUEST_TIMEOUT)
        .send()
        .await
        .context("requesting PaneFleet releases from GitHub")?
        .error_for_status()
        .context("GitHub rejected the PaneFleet releases request")?;
    let releases: Vec<GitHubRelease> = response
        .json()
        .await
        .context("decoding PaneFleet releases from GitHub")?;

    select_available_release(&releases, current_tag)
}

fn select_available_release(
    releases: &[GitHubRelease],
    current_tag: Option<&str>,
) -> Result<Option<PaneFleetRelease>> {
    let is_development_build = current_tag.is_none();
    let current_version = current_tag
        .map(parse_version)
        .transpose()?
        .unwrap_or_else(|| Version::new(0, 0, 0));
    let allow_prerelease = is_development_build || !current_version.pre.is_empty();

    Ok(releases
        .iter()
        .filter(|release| !release.draft)
        .filter_map(|release| {
            let version = parse_version(&release.tag_name).ok()?;
            if !allow_prerelease && !version.pre.is_empty() {
                return None;
            }
            release
                .assets
                .iter()
                .find(|asset| is_compatible_asset(&asset.name))?;
            (version > current_version).then_some((version, release))
        })
        .max_by(|(left, _), (right, _)| left.cmp(right))
        .map(|(_, release)| PaneFleetRelease {
            tag_name: release.tag_name.clone(),
            html_url: release.html_url.clone(),
        }))
}

fn parse_version(tag: &str) -> Result<Version> {
    Version::parse(tag.strip_prefix('v').unwrap_or(tag))
        .map_err(|error| anyhow!("invalid PaneFleet version {tag:?}: {error}"))
}

fn is_compatible_asset(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    if !name.ends_with(".zip") || !name.contains("macos") {
        return false;
    }
    match std::env::consts::ARCH {
        "aarch64" => name.contains("arm64") || name.contains("aarch64"),
        "x86_64" => name.contains("x86_64") || name.contains("amd64"),
        arch => name.contains(arch),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release(tag: &str, asset_name: &str) -> GitHubRelease {
        GitHubRelease {
            tag_name: tag.to_owned(),
            html_url: format!("https://github.com/masyanru/panefleet/releases/tag/{tag}"),
            draft: false,
            assets: vec![GitHubAsset {
                name: asset_name.to_owned(),
            }],
        }
    }

    fn compatible_asset() -> &'static str {
        if std::env::consts::ARCH == "aarch64" {
            "PaneFleet-v0.2.0-macos-arm64.zip"
        } else {
            "PaneFleet-v0.2.0-macos-x86_64.zip"
        }
    }

    #[test]
    fn alpha_build_receives_newer_alpha() {
        let releases = vec![release("v0.1.0-alpha.2", compatible_asset())];
        let selected = select_available_release(&releases, Some("v0.1.0-alpha.1")).unwrap();
        assert_eq!(
            selected.map(|release| release.tag_name),
            Some("v0.1.0-alpha.2".to_owned())
        );
    }

    #[test]
    fn stable_build_ignores_prerelease() {
        let releases = vec![release("v0.2.0-alpha.1", compatible_asset())];
        assert!(
            select_available_release(&releases, Some("v0.1.0"))
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn ignores_drafts_and_incompatible_assets() {
        let mut draft = release("v0.2.0", compatible_asset());
        draft.draft = true;
        let wrong_asset = release("v0.3.0", "PaneFleet-v0.3.0-linux-arm64.zip");
        assert!(
            select_available_release(&[draft, wrong_asset], Some("v0.1.0"))
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn selects_highest_compatible_version() {
        let releases = vec![
            release("v0.1.1", compatible_asset()),
            release("v0.2.0", compatible_asset()),
        ];
        let selected = select_available_release(&releases, Some("v0.1.0")).unwrap();
        assert_eq!(
            selected.map(|release| release.tag_name),
            Some("v0.2.0".to_owned())
        );
    }
}
