//! Detect Cloudflare/auth interstitial pages by title + body markers.
//!
//! Run AFTER navigation, BEFORE extraction or DOM-dump capture. If a known
//! interstitial pattern is matched, return a reason so the caller can bail
//! with a precise error rather than silently capturing a challenge page.

use chromiumoxide::Page;

/// Lowercased substrings that indicate an interstitial. Matched against the
/// page's `document.title` and the first 500 chars of `document.body.innerText`.
pub const INTERSTITIAL_PATTERNS: &[(&str, &str)] = &[
    ("just a moment...", "Cloudflare challenge"),
    ("performing security verification", "Cloudflare challenge"),
    ("checking your browser before", "Cloudflare challenge (v2)"),
    ("verify you are not a bot", "bot challenge"),
    ("attention required", "Cloudflare block"),
    ("sign in to continue", "auth required"),
    ("sign in - google accounts", "Google auth redirect"),
];

/// Returns `Some(reason)` if the current page looks like an interstitial.
/// Errors evaluating the page are swallowed and treated as `None` (we'd
/// rather risk a false negative than block on an evaluation hiccup).
pub async fn detect_interstitial(page: &Page) -> Option<String> {
    let title: String = page
        .evaluate("document.title || ''")
        .await
        .ok()?
        .into_value()
        .ok()?;
    let body: String = page
        .evaluate("(document.body ? document.body.innerText : '').slice(0, 500)")
        .await
        .ok()?
        .into_value()
        .ok()?;
    let title_l = title.to_lowercase();
    let body_l = body.to_lowercase();
    for (needle, reason) in INTERSTITIAL_PATTERNS {
        if title_l.contains(needle) || body_l.contains(needle) {
            return Some(format!("{} (matched '{}')", reason, needle));
        }
    }
    None
}
