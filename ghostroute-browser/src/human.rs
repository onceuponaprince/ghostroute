//! Human-behaviour helpers: jittered sleeps and Unicode-safe input.
//!
//! Each ghostroute CLI duplicated these in three slightly-different forms.
//! Centralising means a fix to (e.g.) the `sanitize_unicode_for_typing`
//! mapping table immediately benefits any CLI that types into the page.

use rand::{rng, RngExt};
use std::time::Duration;
use tokio::time::sleep;

/// Sleep a random duration between `min_ms` and `max_ms` (inclusive). Used
/// for settle waits, pre-extract pauses, and inter-keystroke jitter.
pub async fn human_pause(min_ms: u64, max_ms: u64) {
    let delay = if min_ms >= max_ms {
        min_ms
    } else {
        rng().random_range(min_ms..=max_ms)
    };
    sleep(Duration::from_millis(delay)).await;
}

/// Replace Unicode punctuation that chromiumoxide's `press_key` can't find
/// in its CDP key-definition table with the closest ASCII equivalent.
/// Returns the sanitised text and a flag indicating whether any substitution
/// was made.
///
/// Without this, prompts containing common typographic Unicode (em-dashes,
/// curly quotes, the multiplication sign, geq) error out with
/// `Error: Key not found: <ch>` before the browser even reaches the page.
/// Both `ask-perplexity-cli` and `ask-grok-cli` originally only handled
/// part of this set; centralising prevents drift.
pub fn sanitize_unicode_for_typing(text: &str) -> (String, bool) {
    let mut out = String::with_capacity(text.len());
    let mut changed = false;
    for ch in text.chars() {
        let replacement: Option<&'static str> = match ch {
            // Dashes
            '\u{2014}' | '\u{2013}' | '\u{2212}' => Some("-"),
            // Quotes (curly, low, German low/high)
            '\u{201C}' | '\u{201D}' | '\u{201E}' | '\u{201F}' => Some("\""),
            '\u{2018}' | '\u{2019}' | '\u{201A}' | '\u{201B}' => Some("'"),
            // Ellipsis
            '\u{2026}' => Some("..."),
            // Math operators that bit us in this session
            '\u{00D7}' => Some("x"),       // multiplication sign
            '\u{2260}' => Some("!="),      // not equal
            '\u{2264}' => Some("<="),      // less-than-or-equal
            '\u{2265}' => Some(">="),      // greater-than-or-equal
            '\u{2248}' => Some("~="),      // approximately equal
            // Arrows
            '\u{2192}' | '\u{27A1}' => Some("->"),
            '\u{2190}' | '\u{2B05}' => Some("<-"),
            '\u{2194}' | '\u{2B0C}' => Some("<->"),
            // Section sign (perplexity's CLI failed on this earlier)
            '\u{00A7}' => Some("sec."),
            _ => None,
        };
        match replacement {
            Some(s) => {
                out.push_str(s);
                changed = true;
            }
            None => out.push(ch),
        }
    }
    (out, changed)
}
