//! Shared browser-automation primitives for the ghostroute CLI family
//! (`fast-travel-cli`, `ask-perplexity-cli`, `ask-grok-cli`).
//!
//! Three responsibilities, in roughly the order each launch hits them:
//!   1. [`browser`]: build a stealth-configured chromiumoxide [`Browser`] with
//!      automation fingerprints suppressed and a fixed viewport.
//!   2. [`cookies`]: resolve and inject CDP cookies from
//!      `~/.claude/cookie-configs/<host>-cookies.json`.
//!   3. [`stealth`]: install the `evaluate_on_new_document` overrides that
//!      hide CDP at the JS layer (`navigator.webdriver`, `window.chrome`, ...).
//!
//! Plus runtime helpers used during scraping:
//!   - [`human`]: jittered sleeps + drunk-typist + Unicode sanitisation.
//!   - [`interstitial`]: detect Cloudflare challenges and auth interstitials.
//!   - [`dump`]: rich DOM-shape capture for bootstrapping new providers'
//!     selectors (heuristic Path A from feature-list.md S7).
//!   - [`profile`]: persistent `user_data_dir` mode + interactive
//!     manual-CF-solve handshake (S8).

pub mod browser;
pub mod cookies;
pub mod dump;
pub mod human;
pub mod interstitial;
pub mod profile;
pub mod stealth;

pub use browser::{launch, LaunchOpts};
pub use cookies::{cookie_file_path, load_cookie_data, parse_cookies};
pub use dump::capture_dump_dom;
pub use human::{human_pause, sanitize_unicode_for_typing};
pub use interstitial::detect_interstitial;
pub use profile::{default_profile_dir, ensure_profile_dir, ProfileDir};
pub use stealth::{install_stealth, STEALTH_INIT_SCRIPT};

pub type Result<T> = anyhow::Result<T>;
