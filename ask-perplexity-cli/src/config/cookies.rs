use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};

/// Resolve the Perplexity cookie file. Matches the global convention
/// in ~/.claude/cookie-configs/ used by the Node provider.
pub fn cookie_file_path() -> Result<PathBuf> {
    let home = std::env::var("HOME").context("HOME is not set")?;
    let path = PathBuf::from(home)
        .join(".claude")
        .join("cookie-configs")
        .join("perplexity.ai-cookies.json");
    if !path.exists() {
        bail!(
            "Perplexity cookie file not found at {}. Export cookies via the cookie-master-key extension.",
            path.display()
        );
    }
    Ok(path)
}

pub fn read_cookie_json(path: &Path) -> Result<String> {
    std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read cookie file at {}", path.display()))
}
