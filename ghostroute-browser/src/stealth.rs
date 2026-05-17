//! JS-layer stealth overrides. Must be installed via `evaluate_on_new_document`
//! BEFORE any cookied page load so that Cloudflare's challenge JS sees the
//! overridden values when it runs its bot fingerprint checks.

use anyhow::{Context, Result};
use chromiumoxide::Page;

/// The stealth init script. Mirrors the recipe the broader bot-evasion
/// community has converged on for chromiumoxide/Playwright; matches what
/// `ask-perplexity-cli/src/browser/mod.rs` originally shipped.
pub const STEALTH_INIT_SCRIPT: &str = "\
    Object.defineProperty(navigator, 'webdriver', { get: () => undefined });\
    window.chrome = window.chrome || { runtime: {} };\
    Object.defineProperty(navigator, 'languages', { get: () => ['en-US', 'en'] });\
    Object.defineProperty(navigator, 'plugins', { get: () => [1, 2, 3, 4, 5] });\
";

/// Install [`STEALTH_INIT_SCRIPT`] on a page. Call once per page after
/// creation, before navigating to anything cookied.
pub async fn install_stealth(page: &Page) -> Result<()> {
    page.evaluate_on_new_document(STEALTH_INIT_SCRIPT)
        .await
        .context("Failed to install stealth init script")?;
    Ok(())
}
