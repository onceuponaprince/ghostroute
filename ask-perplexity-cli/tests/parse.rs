use ask_perplexity_cli::parse::{parse, ParseOptions};
use ask_perplexity_cli::types::Phase;
use std::fs;
use std::path::PathBuf;

fn fixture(name: &str) -> String {
    let path = PathBuf::from("tests/fixtures").join(name);
    fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read fixture {}: {}", path.display(), e))
}

#[test]
fn extracts_non_empty_answer_from_auto_web() {
    let html = fixture("auto-web.html");
    let result = parse(
        &html,
        ParseOptions {
            url: Some("https://perplexity.ai/search/abc123"),
            deep: false,
            raw: false,
        },
    )
    .expect("parse should succeed");
    assert!(result.answer.len() > 20);
}

#[test]
fn answer_references_meta_facebook_founder() {
    let html = fixture("auto-web.html");
    let result = parse(&html, ParseOptions { url: None, deep: false, raw: false }).unwrap();
    let lower = result.answer.to_lowercase();
    assert!(
        lower.contains("zuckerberg") || lower.contains("meta") || lower.contains("facebook"),
        "answer did not reference expected founders: {}",
        &result.answer[..result.answer.len().min(200)]
    );
}

#[test]
fn extracts_at_least_one_source_from_auto_web() {
    let html = fixture("auto-web.html");
    let result = parse(&html, ParseOptions { url: None, deep: false, raw: false }).unwrap();
    assert!(!result.sources.is_empty(), "expected ≥1 source in auto-web fixture");
}

#[test]
fn sources_have_expected_fields() {
    let html = fixture("auto-web.html");
    let result = parse(&html, ParseOptions { url: None, deep: false, raw: false }).unwrap();
    for (i, src) in result.sources.iter().enumerate() {
        assert_eq!(src.index, i + 1);
        assert!(!src.title.is_empty());
        assert!(src.url.starts_with("http"));
        assert!(src.domain.contains('.'));
    }
}

#[test]
fn thread_id_extracted_from_search_url() {
    let html = fixture("auto-web.html");
    let result = parse(
        &html,
        ParseOptions {
            url: Some("https://www.perplexity.ai/search/abc-123-uuid"),
            deep: false,
            raw: false,
        },
    )
    .unwrap();
    assert_eq!(result.thread_id.as_deref(), Some("abc-123-uuid"));
}

#[test]
fn thread_id_absent_when_no_search_segment() {
    let html = fixture("auto-web.html");
    let result = parse(
        &html,
        ParseOptions {
            url: Some("https://www.perplexity.ai/"),
            deep: false,
            raw: false,
        },
    )
    .unwrap();
    assert!(result.thread_id.is_none());
}

#[test]
fn deep_research_extracts_steps_with_phases() {
    let html = fixture("deep-research-web.html");
    let result = parse(&html, ParseOptions { url: None, deep: true, raw: false }).unwrap();
    let steps = result.steps.expect("deep: true should populate steps");
    assert!(!steps.is_empty(), "expected ≥1 step");
    let valid = [Phase::Identifying, Phase::Searching, Phase::Insights, Phase::Other];
    for step in &steps {
        assert!(!step.query.is_empty());
        assert!(valid.contains(&step.phase));
    }
}

#[test]
fn non_deep_mode_has_no_steps_field() {
    let html = fixture("auto-web.html");
    let result = parse(&html, ParseOptions { url: None, deep: false, raw: false }).unwrap();
    assert!(result.steps.is_none());
}

#[test]
fn raw_true_populates_answer_html() {
    let html = fixture("auto-web.html");
    let result = parse(&html, ParseOptions { url: None, deep: false, raw: true }).unwrap();
    let raw = result.raw.expect("raw: true should populate raw field");
    assert!(!raw.answer_html.is_empty());
}

#[test]
fn raw_false_omits_raw_field() {
    let html = fixture("auto-web.html");
    let result = parse(&html, ParseOptions { url: None, deep: false, raw: false }).unwrap();
    assert!(result.raw.is_none());
}
