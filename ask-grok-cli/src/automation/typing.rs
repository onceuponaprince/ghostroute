use anyhow::Result;
use chromiumoxide::cdp::browser_protocol::input::InsertTextParams;
use chromiumoxide::{Element, Page};
use rand::RngExt;
use std::time::Duration;
use tokio::time::sleep;

pub async fn human_pause(min_ms: u64, max_ms: u64) {
    let mut rng = rand::rng();
    let delay = if min_ms >= max_ms {
        min_ms
    } else {
        rng.random_range(min_ms..=max_ms)
    };
    sleep(Duration::from_millis(delay)).await;
}

/// Insert text into the currently-focused element via CDP `Input.insertText`.
/// Bypasses chromiumoxide's per-char synthetic-KeyEvent path, which only
/// covers a fixed keymap and errors on Unicode like `→`, `—`, `≥`. The DOM
/// receives `input` events but no `keydown`/`keyup` — sufficient for chat
/// inputs that listen for `oninput`. Caller is responsible for ensuring the
/// target element has focus (e.g. via a prior click).
async fn insert_text(page: &Page, text: &str) -> Result<()> {
    page.execute(InsertTextParams::new(text.to_string())).await?;
    Ok(())
}

/// Some chat UIs (Grok runs on tiptap/ProseMirror) only mark the message as
/// "submittable" after they observe an `input` event AND a synthetic React
/// state update. `Input.insertText` fires the native `input` event, but
/// React's controlled-input pattern can sit on a stale empty-string value
/// until something forces a re-read. Dispatching `input` again on the
/// active element is a cheap belt-and-braces measure.
async fn nudge_input_event(page: &Page) -> Result<()> {
    page.evaluate(
        r#"(() => {
            const el = document.activeElement;
            if (!el) return false;
            el.dispatchEvent(new InputEvent('input', { bubbles: true, cancelable: true }));
            return true;
        })()"#,
    )
    .await?;
    Ok(())
}

/// Whole-string instant paste — used for re-injecting prior context so the
/// model sees its history without watching us re-type 5,000 characters.
pub async fn paste_text(page: &Page, text: &str) -> Result<()> {
    insert_text(page, text).await?;
    let _ = nudge_input_event(page).await;
    Ok(())
}

pub async fn human_type_with_typos(page: &Page, chat_input: &Element, text: &str) -> Result<()> {
    let typos = ['s', 'd', 'f', 'g', 'x', 'c'];
    let mut rng = rand::rng();

    for (idx, c) in text.chars().enumerate() {
        // Newlines would submit the chat; tabs would shift focus out. Coerce
        // both to space so cadence holds and content stays in the field.
        let c = if c == '\n' || c == '\r' || c == '\t' { ' ' } else { c };

        if idx > 0 {
            let delay = rng.random_range(30..150);
            sleep(Duration::from_millis(delay)).await;
        }

        let mistake_roll = rng.random_range(1..=100);
        if mistake_roll <= 5 {
            let bad_char = typos[rng.random_range(0..typos.len())];
            insert_text(page, &bad_char.to_string()).await?;
            sleep(Duration::from_millis(250)).await;
            chat_input.press_key("Backspace").await?;
            sleep(Duration::from_millis(100)).await;
        }

        insert_text(page, &c.to_string()).await?;
    }

    Ok(())
}
