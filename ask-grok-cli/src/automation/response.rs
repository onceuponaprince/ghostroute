use anyhow::{bail, Context, Result};
use chromiumoxide::Page;
use std::time::{Duration, Instant};
use tokio::time::sleep;

use super::probe::probe_response_state;

pub struct ResponseDetails {
    pub answer: String,
    pub paragraph_count: usize,
    pub char_count: usize,
    pub preview: String,
}

fn preview_text(text: &str, max_chars: usize) -> String {
    let mut out = String::new();
    for (i, ch) in text.chars().enumerate() {
        if i >= max_chars {
            out.push_str("...");
            break;
        }
        out.push(ch);
    }
    out
}

/// Whitespace normaliser. Browser's `innerText` preserves `\n` between block
/// elements (paragraphs, lists), so a raw `!=` against the already-collapsed
/// prompt always passes and the user bubble gets returned as if it were
/// Grok's reply. Both sides must collapse.
fn normalise_ws(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Pull the latest response candidate from the page in a single CDP round-trip.
///
/// Force-opens any closed `<details>` first — `innerText` follows the browser
/// spec and skips closed-`<details>` content, which dropped Grok's bullets-
/// under-headers (Survivors:, Rollbacks:) from the captured reply. With them
/// open, every visible block contributes to the returned text.
///
/// Two-element gate is enforced in JS (matches the prior Rust loop): the user
/// bubble and the assistant bubble share the same selector, so a single match
/// is by definition the user echo, not yet a response.
async fn extract_response_candidate(
    page: &Page,
    selector: &str,
    prompt_trimmed: &str,
) -> Option<String> {
    let selector_literal = serde_json::to_string(selector).ok()?;
    let prompt_literal = serde_json::to_string(prompt_trimmed).ok()?;
    let js = format!(
        r#"(() => {{
          const els = Array.from(document.querySelectorAll({selector_literal}));
          if (els.length < 2) return '';
          const normalise = (s) => s.split(/\s+/).filter(Boolean).join(' ');
          for (let i = els.length - 1; i >= 0; i--) {{
            const el = els[i];
            el.querySelectorAll('details:not([open])').forEach((d) => (d.open = true));
            const text = (el.innerText || '').trim();
            if (text && normalise(text) !== {prompt_literal}) {{
              return text;
            }}
          }}
          return '';
        }})()"#
    );
    let value = page.evaluate(js).await.ok()?;
    value.into_value::<String>().ok()
}

/// Count block-level content (`<p>` + `<li>`) in the latest response container.
/// Bullets are first-class content for Grok responses; counting only `<p>`
/// understated how much of the reply is structured material.
async fn count_response_blocks(page: &Page, selector: &str) -> usize {
    let Ok(selector_literal) = serde_json::to_string(selector) else {
        return 0;
    };
    let js = format!(
        r#"(() => {{
          const els = document.querySelectorAll({selector_literal});
          if (!els.length) return 0;
          const last = els[els.length - 1];
          return last.querySelectorAll('p, li').length;
        }})()"#
    );
    let Ok(value) = page.evaluate(js).await else {
        return 0;
    };
    value.into_value::<f64>().map(|n| n as usize).unwrap_or(0)
}

async fn wait_for_stable_response_text(
    page: &Page,
    response_selector: &str,
    prompt: &str,
    timeout_ms: u64,
) -> Result<String> {
    let started = Instant::now();
    // Match the form the browser's innerText actually returns: any consecutive
    // whitespace (including \n / \r / \t / multiple spaces) collapses to a
    // single space. Drunk-Typist strips \n→space individually but leaves \n\n
    // as two spaces; the browser then collapses them to one. Without this
    // normalisation, the user's prompt bubble passes the not-the-prompt filter
    // and gets returned as Grok's response.
    let prompt_trimmed: String = prompt.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut last_candidate = String::new();
    let mut stable_ticks = 0_u8;

    loop {
        let candidate = extract_response_candidate(page, response_selector, &prompt_trimmed)
            .await
            .unwrap_or_default();
        let candidate = candidate.trim();

        if !candidate.is_empty() && normalise_ws(candidate) != prompt_trimmed {
            if candidate == last_candidate {
                stable_ticks += 1;
            } else {
                last_candidate = candidate.to_string();
                stable_ticks = 0;
            }

            if stable_ticks >= 2 {
                return Ok(last_candidate);
            }
        }

        if started.elapsed() >= Duration::from_millis(timeout_ms) {
            break;
        }

        sleep(Duration::from_millis(700)).await;
    }

    if !last_candidate.is_empty() {
        return Ok(last_candidate);
    }

    if let Some(probe) = probe_response_state(page, response_selector).await {
        eprintln!(
            "[probe] response state at stability-wait timeout (selector='{response_selector}'):\n{probe}"
        );
    } else {
        eprintln!("[probe] response-state probe itself failed — page may be unresponsive");
    }

    bail!(
        "Timed out waiting for stable response text (timeout={}ms)",
        timeout_ms
    );
}

pub async fn collect_response_details(
    page: &Page,
    response_selector: &str,
    prompt: &str,
    timeout_ms: u64,
) -> Result<ResponseDetails> {
    // wait_for_stable_response_text now force-opens <details> and reads the
    // full innerText of the assistant turn, including bullets. The previous
    // <p>-only walk that built `merged` here dropped <li> siblings of <p>
    // entirely — that walk is gone. Stable text is canonical.
    let answer = wait_for_stable_response_text(page, response_selector, prompt, timeout_ms)
        .await
        .context("Failed while waiting for stable response text")?;

    let paragraph_count = count_response_blocks(page, response_selector).await;

    Ok(ResponseDetails {
        char_count: answer.chars().count(),
        preview: preview_text(&answer, 240),
        paragraph_count,
        answer,
    })
}
