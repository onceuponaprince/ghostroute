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
    let config = builder
        .arg("--no-sandbox")
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
