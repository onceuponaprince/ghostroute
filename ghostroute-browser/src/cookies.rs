//! Cookie file resolution + CDP-shape injection.
//!
//! Convention across the ghostroute CLIs and the cookie-master-key Chrome
//! extension that exports them: each provider's cookies live at
//! `~/.claude/cookie-configs/<host>-cookies.json` as a JSON array of CDP
//! [`CookieParam`] objects.

use anyhow::{bail, Context, Result};
use chromiumoxide::cdp::browser_protocol::network::CookieParam;
use std::fs;
use std::path::{Path, PathBuf};

/// Default location for a provider's cookie export.
pub fn cookie_file_path(host: &str) -> Result<PathBuf> {
    let home = std::env::var("HOME").context("HOME is not set")?;
    let path = PathBuf::from(home)
        .join(".claude")
        .join("cookie-configs")
        .join(format!("{host}-cookies.json"));
    Ok(path)
}

/// Read a cookie file. Returns the raw JSON string for callers that want to
/// pass it through unchanged; use [`parse_cookies`] for the structured form.
pub fn load_cookie_data(path: &Path) -> Result<String> {
    if !path.exists() {
        bail!(
            "Cookie file not found at {}. Export via cookie-master-key from a logged-in tab.",
            path.display()
        );
    }
    fs::read_to_string(path)
        .with_context(|| format!("Failed to read cookie file {}", path.display()))
}

/// Parse a cookie-file's JSON into CDP [`CookieParam`] entries.
pub fn parse_cookies(json: &str) -> Result<Vec<CookieParam>> {
    serde_json::from_str(json).context("Cookie file is not valid CDP cookie JSON")
}
