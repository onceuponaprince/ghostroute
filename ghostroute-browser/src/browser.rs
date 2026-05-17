//! Browser launch with stealth flags, fixed viewport, and optional persistent
//! `user_data_dir`. Setting a unique `user_data_dir` per launch (which is the
//! default) also unblocks **multiple browsers in parallel** — chromiumoxide's
//! shared `/tmp/chromiumoxide-runner/SingletonLock` is the reason ghostroute
//! CLIs collided when run concurrently. Each unique dir is its own lock.

use anyhow::{Context, Result};
use chromiumoxide::handler::viewport::Viewport;
use chromiumoxide::{Browser, BrowserConfig};
use futures::StreamExt;
use std::path::PathBuf;

/// Tunables for [`launch`]. Defaults are headless ephemeral with stealth on.
#[derive(Debug, Clone, Default)]
pub struct LaunchOpts {
    /// Show a real Chromium window. Required to bypass Cloudflare's
    /// interactive challenge on first solve; once a profile retains
    /// `cf_clearance`, headless typically works.
    pub visible: bool,

    /// Persistent profile directory. When set, cookies/storage/cache survive
    /// across runs — exactly what we need to keep `cf_clearance` between
    /// invocations. When `None`, chromiumoxide uses an ephemeral temp dir
    /// per launch (also avoiding SingletonLock collisions in parallel runs).
    pub profile_dir: Option<PathBuf>,

    /// Extra Chromium flags appended after the stealth defaults.
    pub extra_args: Vec<String>,
}

/// Launch a stealth-configured Chromium browser. Returns the [`Browser`]
/// and the join handle for its event-stream drain (must be kept alive).
pub async fn launch(opts: LaunchOpts) -> Result<(Browser, tokio::task::JoinHandle<()>)> {
    let mut builder = BrowserConfig::builder()
        .arg("--no-sandbox")
        // Most-known automation tells, suppressed:
        .arg("--disable-blink-features=AutomationControlled")
        // Drop chromiumoxide's auto-injected --enable-automation, which
        // Cloudflare's bot fingerprint specifically looks for. Then restore
        // only the small set of flags the rest of the system actually needs.
        .disable_default_args()
        .arg("--enable-logging=stderr")
        .arg("--disable-background-networking")
        // Real-laptop viewport; chromiumoxide's default is mobile-ish.
        .viewport(Some(Viewport {
            width: 1440,
            height: 900,
            device_scale_factor: None,
            emulating_mobile: false,
            is_landscape: false,
            has_touch: false,
        }));

    if opts.visible {
        builder = builder.with_head();
    }
    if let Some(dir) = opts.profile_dir.as_ref() {
        builder = builder.user_data_dir(dir.clone());
    }
    for arg in opts.extra_args {
        builder = builder.arg(arg);
    }

    let config = builder
        .build()
        .map_err(|e| anyhow::anyhow!("Failed to build BrowserConfig: {}", e))?;

    let (browser, mut handler) = Browser::launch(config)
        .await
        .context("Failed to launch Chromium via chromiumoxide")?;

    let handle = tokio::spawn(async move {
        while handler.next().await.is_some() {}
    });

    Ok((browser, handle))
}
