use anyhow::{anyhow, Context, Result};
use chromiumoxide::Page;
use std::time::Duration;
use tokio::time::timeout;

use crate::automation::human::{human_click, human_pause};
use crate::config::{
    focus_url, model_label, tool_label, LOGIN_WALL_HREF_SUBSTRING, MODEL_BUTTON, PROMPT_INPUT,
    SOURCES_BUTTON_TEXT, TOOLS_BUTTON,
};

pub async fn navigate_focus_and_thread(
    page: &Page,
    focus: &str,
    thread: Option<&str>,
) -> Result<()> {
    let url = match thread {
        Some(t) => format!("https://www.perplexity.ai/search/{}", t),
        None => focus_url(focus).to_string(),
    };
    page.goto(&url)
        .await
        .with_context(|| format!("Failed to navigate to {url}"))?;
    human_pause(2_000, 3_500).await;

    // Fail fast if we landed on a login wall.
    let signin = page
        .find_element(&format!(r#"a[href*="{}"]"#, LOGIN_WALL_HREF_SUBSTRING))
        .await;
    if signin.is_ok() {
        return Err(anyhow!(
            "Perplexity login wall detected — refresh cookies at ~/.claude/cookie-configs/perplexity.ai-cookies.json"
        ));
    }
    Ok(())
}

/// Opens the Model menu and picks the item whose visible label contains
/// `label`. Silently skips when the Model button isn't visible (topic
/// routes like /academic don't expose it).
pub async fn select_model(page: &Page, model: &str) -> Result<()> {
    let Some(label) = model_label(model) else {
        return Err(anyhow!("unknown model: {model}"));
    };
    let Ok(Ok(btn)) = timeout(Duration::from_secs(2), page.find_element(MODEL_BUTTON)).await else {
        // Model button not visible on this page (focus routes) — skip.
        return Ok(());
    };
    human_click(page, &btn).await?;
    human_pause(400, 900).await;

    // Find the menuitemradio whose text contains `label`. chromiumoxide
    // doesn't support :has-text(); do the text filter in JS.
    let selector = format!(
        r#"[role="menuitemradio"]"#
    );
    let picker = page
        .evaluate(format!(
            "(() => {{ const nodes = document.querySelectorAll('{selector}');\
             for (const n of nodes) {{ if (n.textContent && n.textContent.includes('{label}')) return true; }} return false; }})()"
        ))
        .await?;
    let found: bool = picker.into_value().unwrap_or(false);
    if !found {
        return Err(anyhow!("Model menuitem with label \"{label}\" not found"));
    }
    // Click by evaluating — simpler than locating + clicking via Element.
    page.evaluate(format!(
        "(() => {{ const nodes = document.querySelectorAll('{selector}');\
         for (const n of nodes) {{ if (n.textContent && n.textContent.includes('{label}')) {{ n.click(); return true; }} }} return false; }})()"
    ))
    .await?;
    human_pause(200, 500).await;
    Ok(())
}

/// Opens the Tools menu (+ button) and picks the named tool.
pub async fn select_tool(page: &Page, tool: Option<&str>) -> Result<()> {
    let Some(tool) = tool else {
        return Ok(());
    };
    let Some(label) = tool_label(tool) else {
        return Err(anyhow!("unknown tool: {tool}"));
    };
    let btn = page
        .find_element(TOOLS_BUTTON)
        .await
        .context("Tools (+) button not found")?;
    human_click(page, &btn).await?;
    human_pause(400, 900).await;

    let selector = r#"[role="menuitemradio"]"#;
    let clicked = page
        .evaluate(format!(
            "(() => {{ const nodes = document.querySelectorAll('{selector}');\
             for (const n of nodes) {{ if (n.textContent && n.textContent.includes('{label}')) {{ n.click(); return true; }} }} return false; }})()"
        ))
        .await?
        .into_value::<bool>()
        .unwrap_or(false);
    if !clicked {
        return Err(anyhow!("Tools menuitem with label \"{label}\" not found"));
    }
    human_pause(200, 500).await;
    Ok(())
}

/// Open the Sources overlay (click the "N sources" button) so parse.js
/// can extract the source cards from the returned HTML. Silently skips
/// when the button isn't present (query didn't browse the web).
pub async fn open_sources_overlay(page: &Page) -> Result<()> {
    let clicked = page
        .evaluate(format!(
            "(() => {{ const btns = document.querySelectorAll('button');\
             for (const b of btns) {{ if (b.textContent && b.textContent.toLowerCase().includes('{SOURCES_BUTTON_TEXT}')) {{ b.click(); return true; }} }} return false; }})()"
        ))
        .await?
        .into_value::<bool>()
        .unwrap_or(false);
    if clicked {
        human_pause(600, 1_200).await;
    }
    Ok(())
}

/// Submit a prompt by focusing the prompt input, humanly typing the text,
/// then pressing Enter. Caller must have already selected model + tool.
pub async fn submit_prompt(page: &Page, prompt: &str) -> Result<()> {
    let input = page
        .find_element(PROMPT_INPUT)
        .await
        .context("Prompt input not found")?;
    human_click(page, &input).await?;
    crate::automation::human::human_type(&input, prompt).await?;
    human_pause(200, 500).await;
    input.press_key("Enter").await?;
    Ok(())
}

use std::time::Instant;

const FAST_TOTAL_MS: u64 = 180_000;
const DEEP_TOTAL_MS: u64 = 1_800_000;
const FAST_FIRST_RENDER_MS: u64 = 60_000;
const DEEP_FIRST_RENDER_MS: u64 = 300_000;
const POLL_INTERVAL_MS: u64 = 800;

pub async fn wait_for_answer_stable(
    page: &Page,
    deep: bool,
    mut on_progress: Option<Box<dyn FnMut(String) + Send>>,
) -> Result<()> {
    let first_render = if deep { DEEP_FIRST_RENDER_MS } else { FAST_FIRST_RENDER_MS };
    let stable_ms: u64 = if deep { 15_000 } else { 3_000 };
    let total_ms = if deep { DEEP_TOTAL_MS } else { FAST_TOTAL_MS };

    // Wait for the first markdown-content div to appear.
    let appeared = poll_for_answer_container(page, first_render).await?;
    if !appeared {
        return Err(anyhow!(
            "answer-not-rendered after {}ms (deep={deep})",
            first_render
        ));
    }

    let start = Instant::now();
    let mut last_len: usize = 0;
    let mut last_change = Instant::now();
    let mut last_phase_count: usize = 0;

    while start.elapsed().as_millis() < total_ms as u128 {
        let len = read_last_answer_length(page).await.unwrap_or(0);
        if len != last_len {
            last_len = len;
            last_change = Instant::now();
        }
        if let Some(cb) = on_progress.as_mut() {
            let count = count_answer_containers(page).await.unwrap_or(0);
            if count > last_phase_count {
                last_phase_count = count;
                cb(format!("phase {count}"));
            }
        }
        if len > 0 && last_change.elapsed().as_millis() >= stable_ms as u128 {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(POLL_INTERVAL_MS)).await;
    }
    Err(anyhow!("answer-stable-total timeout after {total_ms}ms"))
}

async fn poll_for_answer_container(page: &Page, timeout_ms: u64) -> Result<bool> {
    let deadline = Instant::now() + Duration::from_millis(timeout_ms);
    while Instant::now() < deadline {
        let found = count_answer_containers(page).await.unwrap_or(0) > 0;
        if found {
            return Ok(true);
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    Ok(false)
}

async fn count_answer_containers(page: &Page) -> Result<usize> {
    let v = page
        .evaluate("document.querySelectorAll('div[id^=\"markdown-content-\"]').length")
        .await?;
    Ok(v.into_value::<i64>().unwrap_or(0) as usize)
}

async fn read_last_answer_length(page: &Page) -> Result<usize> {
    let v = page
        .evaluate(
            "(() => { const nodes = document.querySelectorAll('div[id^=\"markdown-content-\"]'); \
             if (nodes.length === 0) return 0; \
             return (nodes[nodes.length - 1].textContent || '').length; })()",
        )
        .await?;
    Ok(v.into_value::<i64>().unwrap_or(0) as usize)
}

/// Orchestrates a single scrape: navigate → select model/tool → submit →
/// wait → open sources → return HTML + final URL.
pub async fn scrape_once(
    page: &Page,
    prompt: &str,
    model: &str,
    tool: Option<&str>,
    focus: &str,
    thread: Option<&str>,
    on_progress: Option<Box<dyn FnMut(String) + Send>>,
) -> Result<(String, String)> {
    navigate_focus_and_thread(page, focus, thread).await?;
    select_model(page, model).await?;
    select_tool(page, tool).await?;
    submit_prompt(page, prompt).await?;

    let deep = tool == Some("deep-research");
    wait_for_answer_stable(page, deep, on_progress).await?;

    if deep {
        human_pause(2_000, 4_000).await;
    }
    open_sources_overlay(page).await?;

    let html = page
        .content()
        .await
        .context("Failed to capture page HTML")?;
    let url = page.url().await?.unwrap_or_default();
    Ok((html, url))
}
