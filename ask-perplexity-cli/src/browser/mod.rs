use anyhow::{Context, Result};
use chromiumoxide::cdp::browser_protocol::network::CookieParam;
use chromiumoxide::{Browser, BrowserConfig, Page};
use futures::StreamExt;

/// Launches a headed Chromium with the stealth flags required to bypass
/// Perplexity's Pro-feature gate and Cloudflare. Matches the Node provider's
/// `launchAndNavigate` setup:
///   - `headless: false` — Cloudflare detects the flag itself
///   - `--disable-blink-features=AutomationControlled` + skip
///     `--enable-automation` — suppress automation fingerprint
///   - After navigation, inject an init script that overrides
///     `navigator.webdriver` and stubs `window.chrome`.
pub async fn bootstrap_browser(cookie_json: &str) -> Result<(Browser, Page)> {
    let config = BrowserConfig::builder()
        .with_head()
        .arg("--no-sandbox")
        .arg("--disable-blink-features=AutomationControlled")
        .disable_default_args()
        // With defaults disabled, restore the minimum necessary ones but
        // without --enable-automation:
        .arg("--enable-logging=stderr")
        .arg("--disable-background-networking")
        .viewport(Some(chromiumoxide::handler::viewport::Viewport {
            width: 1440,
            height: 900,
            device_scale_factor: None,
            emulating_mobile: false,
            is_landscape: false,
            has_touch: false,
        }))
        .build()
        .map_err(|e| anyhow::anyhow!("Failed to build BrowserConfig: {}", e))?;

    let (browser, mut handler) = Browser::launch(config)
        .await
        .context("Failed to launch Chromium via chromiumoxide")?;

    tokio::spawn(async move {
        while handler.next().await.is_some() {}
    });

    let cookies: Vec<CookieParam> =
        serde_json::from_str(cookie_json).context("Cookie file is not valid CDP cookie JSON")?;

    // Navigate to an initial page so set_cookies has a domain context.
    let page = browser
        .new_page("https://www.perplexity.ai")
        .await
        .context("Failed to navigate to perplexity.ai")?;

    // Install the stealth init script BEFORE any more navigation so the
    // overrides apply on the next page load.
    page.evaluate_on_new_document(
        "Object.defineProperty(navigator, 'webdriver', { get: () => undefined });\
         window.chrome = window.chrome || { runtime: {} };\
         Object.defineProperty(navigator, 'languages', { get: () => ['en-US', 'en'] });",
    )
    .await
    .context("Failed to install stealth init script")?;

    page.set_cookies(cookies)
        .await
        .context("Failed to inject cookies")?;

    page.reload().await.context("Failed to reload after cookie injection")?;

    Ok((browser, page))
}
