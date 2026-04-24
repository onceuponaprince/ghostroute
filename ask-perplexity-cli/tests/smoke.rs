//! Live integration smoke test — requires valid Perplexity cookies at
//! ~/.claude/cookie-configs/perplexity.ai-cookies.json. Runs a real browser
//! and submits a prompt to the real perplexity.ai. Marked `#[ignore]` so
//! `cargo test` skips it by default; run with:
//!     cargo test --test smoke -- --ignored --nocapture

use ask_perplexity_cli::automation::scrape::scrape_once;
use ask_perplexity_cli::browser::bootstrap_browser;
use ask_perplexity_cli::config::cookies::{cookie_file_path, read_cookie_json};
use ask_perplexity_cli::parse::{parse, ParseOptions};

#[tokio::test]
#[ignore]
async fn live_fast_path_returns_answer_with_sources() {
    let cookie_path = cookie_file_path().expect("cookies missing — skip this smoke if offline");
    let cookie_json = read_cookie_json(&cookie_path).expect("cookie read");
    let (mut browser, page) = bootstrap_browser(&cookie_json)
        .await
        .expect("browser bootstrap");

    let (html, url) = scrape_once(
        &page,
        "who founded meta (formerly facebook)?",
        "best",
        None,
        "web",
        None,
        None,
    )
    .await
    .expect("scrape");

    drop(page);
    browser.close().await.ok();

    let result = parse(
        &html,
        ParseOptions {
            url: Some(&url),
            deep: false,
            raw: false,
        },
    )
    .expect("parse");
    assert!(result.answer.len() > 20, "answer too short: {}", result.answer);
    assert!(!result.sources.is_empty(), "no sources extracted");
    assert!(result.thread_id.is_some(), "no threadId extracted");
}
