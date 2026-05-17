//! Persistent browser profile management (S8). Once a profile completes a
//! Cloudflare challenge interactively, its `cf_clearance` is retained in the
//! `user_data_dir` for future headless runs.
//!
//! Two flavours of profile:
//!   - **Per-provider** (`provider/<name>`): isolates cookies/state between
//!     providers, avoids cross-contamination.
//!   - **Multi-provider** (`shared/<label>`): one profile that holds tabs/
//!     cookies for several providers. Used by `--init-all` to log in across
//!     LLMs in one Chromium session.

use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

/// Newtype wrapper so the profile-dir contract is explicit at call sites.
#[derive(Debug, Clone)]
pub struct ProfileDir(pub PathBuf);

impl ProfileDir {
    pub fn as_path(&self) -> &Path {
        &self.0
    }
    pub fn into_inner(self) -> PathBuf {
        self.0
    }
}

/// Default location for ghostroute profiles. Put them under XDG cache so
/// they're regenerable; the cookies/state inside aren't precious enough to
/// belong in `~/.config`.
pub fn default_profile_dir(label: &str) -> Result<ProfileDir> {
    let home = std::env::var("HOME").context("HOME is not set")?;
    let path = PathBuf::from(home)
        .join(".cache")
        .join("ghostroute")
        .join("profiles")
        .join(label);
    Ok(ProfileDir(path))
}

/// Create the directory if it doesn't exist. chromiumoxide will populate it
/// on first launch with whatever Chrome wants to keep there.
pub fn ensure_profile_dir(dir: &ProfileDir) -> Result<()> {
    fs::create_dir_all(dir.as_path()).with_context(|| {
        format!(
            "Failed to create profile directory at {}",
            dir.as_path().display()
        )
    })?;
    Ok(())
}
