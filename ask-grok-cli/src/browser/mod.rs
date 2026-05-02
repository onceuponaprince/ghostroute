use anyhow::{Context, Result};
use chromiumoxide::cdp::browser_protocol::network::CookieParam;
use chromiumoxide::{Browser, BrowserConfig, Page};
use futures::StreamExt;

pub async fn bootstrap_browser_session(
    cookie_data: &str,
    headless: bool,
) -> Result<(Browser, Page)> {
    let mut builder = BrowserConfig::builder();
    if !headless {
        builder = builder.with_head();
    }
    // Per-pid profile dir: chromiumoxide's default temp dir gets locked between
    // runs and second invocations exit cleanly with empty stderr (the historic
    // 2026-04-27 failure mode). Unique per-pid sidesteps the lock entirely.
    let user_data_dir = format!("/tmp/grok-{}", std::process::id());
    let config = builder
        .arg("--no-sandbox")
        // /dev/shm is 64MB by default in many envs; Grok's React tree blows past
        // it and the renderer dies silently mid-session. /tmp has room.
        .arg("--disable-dev-shm-usage")
        // GPU process death cascades into renderer death in headless. Kill it.
        .arg("--disable-gpu")
        // navigator.webdriver=true is the cheapest tell. xAI's anti-bot reads it.
        .arg("--disable-blink-features=AutomationControlled")
        .arg(format!("--user-data-dir={}", user_data_dir))
        .build()
        .map_err(|e| anyhow::anyhow!("Failed to build BrowserConfig: {}", e))?;

    let (browser, mut handler) = Browser::launch(config)
        .await
        .context("Failed to launch Chromium via chromiumoxide")?;

    // The handler stream MUST be polled or every command hangs indefinitely.
    // Matches chromiumoxide's canonical examples/storage-cookie pattern: drain forever,
    // never break on error — individual CDP errors don't mean the connection is dead.
    tokio::spawn(async move {
        while handler.next().await.is_some() {}
    });

    let cookies: Vec<CookieParam> = serde_json::from_str(&cookie_data)
        .context("Cookie file is not valid CDP cookie JSON")?;

    // new_page both creates and navigates. Going to grok.com first gives the page a
    // real domain context — CDP rejects set_cookies on about:blank.
    let page = browser
        .new_page("https://grok.com")
        .await
        .context("Failed to navigate to grok.com")?;

    page.set_cookies(cookies)
        .await
        .context("Failed to inject cookies")?;

    // Reload so the authenticated session is picked up on load.
    page.reload()
        .await
        .context("Failed to reload after cookie injection")?;

    Ok((browser, page))
}
