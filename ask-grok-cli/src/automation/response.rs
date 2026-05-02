use anyhow::{bail, Context, Result};
use chromiumoxide::{Element, Page};
use std::time::{Duration, Instant};
use tokio::time::sleep;

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

async fn element_inner_text(element: &Element) -> String {
    element
        .inner_text()
        .await
        .ok()
        .flatten()
        .unwrap_or_default()
}

/// Whitespace normaliser shared by both response filters. Browser's `innerText`
/// preserves `\n` between block elements (paragraphs, lists), so a raw `!=`
/// against the already-collapsed prompt always passes and the user bubble
/// gets returned as if it were Grok's reply. Both sides must collapse.
fn normalise_ws(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
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
        let mut newest_candidate = String::new();
        if let Ok(elements) = page.find_elements(response_selector).await {
            // Grok's `[id^="response-"] .message-bubble` matches BOTH the user
            // bubble (typed prompt) and the assistant bubble (Grok's reply).
            // With a single match, the only thing to read is the user echo —
            // which is by definition not yet a response. Wait for at least
            // two matching elements before accepting any candidate.
            if elements.len() >= 2 {
                for element in elements.into_iter().rev() {
                    let text = element_inner_text(&element).await;
                    let candidate = text.trim();
                    if !candidate.is_empty() && normalise_ws(candidate) != prompt_trimmed {
                        newest_candidate = candidate.to_string();
                        break;
                    }
                }
            }
        }

        let candidate = newest_candidate.trim();

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
    // Wait for stable text BEFORE counting <p>s. The container can become visible
    // a beat before its paragraph children finish rendering during streaming —
    // bailing on an empty <p> count here would fire a false negative.
    let raw_stable_text = wait_for_stable_response_text(page, response_selector, prompt, timeout_ms)
        .await
        .context("Failed while waiting for stable response text")?;

    let response_element = page
        .find_elements(response_selector)
        .await
        .unwrap_or_default()
        .into_iter()
        .rev()
        .next();

    let paragraphs = if let Some(element) = response_element.as_ref() {
        element.find_elements("p").await.unwrap_or_default()
    } else {
        Vec::new()
    };
    let paragraph_count = paragraphs.len();

    let mut paragraph_texts = Vec::new();
    for p in &paragraphs {
        if let Ok(Some(t)) = p.inner_text().await {
            paragraph_texts.push(t);
        }
    }

    let merged = paragraph_texts
        .into_iter()
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n");

    let answer = if !merged.is_empty() && normalise_ws(&merged) != normalise_ws(prompt) {
        merged
    } else {
        raw_stable_text
    };

    Ok(ResponseDetails {
        paragraph_count,
        char_count: answer.chars().count(),
        preview: preview_text(&answer, 240),
        answer,
    })
}
