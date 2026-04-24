use anyhow::Result;
use chromiumoxide::Element;
use rand::{rng, RngExt};
use std::time::Duration;
use tokio::time::sleep;

/// Sleep a random duration between `min_ms` and `max_ms` (inclusive).
pub async fn human_pause(min_ms: u64, max_ms: u64) {
    let delay = if min_ms >= max_ms {
        min_ms
    } else {
        rng().random_range(min_ms..=max_ms)
    };
    sleep(Duration::from_millis(delay)).await;
}

/// Replace Unicode punctuation that chromiumoxide's `press_key` can't
/// find in its key-definition table with the closest ASCII equivalent.
/// Returns the sanitized string and a flag indicating whether any
/// substitution was made. Without this, a prompt containing an em-dash
/// like "A — B" errors out with "Key not found: —" before the browser
/// even reaches Perplexity. Playwright handles this transparently via
/// `Input.insertText`; chromiumoxide 0.7 dispatches per-char KeyEvents.
pub fn sanitize_for_typing(text: &str) -> (String, bool) {
    let mut out = String::with_capacity(text.len());
    let mut changed = false;
    for ch in text.chars() {
        let replacement: Option<&'static str> = match ch {
            '\u{2014}' | '\u{2013}' | '\u{2212}' => Some("-"),
            '\u{201C}' | '\u{201D}' | '\u{201E}' | '\u{201F}' => Some("\""),
            '\u{2018}' | '\u{2019}' | '\u{201A}' | '\u{201B}' => Some("'"),
            '\u{2026}' => Some("..."),
            '\u{00A0}' => Some(" "),
            _ => None,
        };
        if let Some(rep) = replacement {
            out.push_str(rep);
            changed = true;
        } else {
            out.push(ch);
        }
    }
    (out, changed)
}

/// Type `text` into `element` with per-char delay jitter, occasional
/// pauses, and a small rate of typo-then-backspace corrections.
/// Mirrors providers/perplexity/human.js#humanType from Plan 1.
pub async fn human_type(element: &Element, text: &str) -> Result<()> {
    let (text_owned, changed) = sanitize_for_typing(text);
    if changed {
        eprintln!(
            "[warn] prompt contained Unicode punctuation chromiumoxide can't type; \
             replaced em/en-dashes -> '-', curly quotes -> straight, ellipsis -> '...'"
        );
    }
    let text = text_owned.as_str();
    const NEARBY: &[(&str, &str)] = &[
        ("a", "s"), ("s", "d"), ("d", "f"), ("f", "g"), ("g", "h"),
        ("h", "j"), ("j", "k"), ("k", "l"), ("q", "w"), ("w", "e"),
        ("e", "r"), ("r", "t"), ("t", "y"), ("y", "u"), ("u", "i"),
        ("i", "o"), ("o", "p"), ("z", "x"), ("x", "c"), ("c", "v"),
        ("v", "b"), ("b", "n"), ("n", "m"),
    ];
    let find_nearby = |c: char| -> Option<char> {
        let lower = c.to_ascii_lowercase().to_string();
        NEARBY
            .iter()
            .find(|(k, _)| *k == lower)
            .map(|(_, v)| {
                if c.is_ascii_uppercase() {
                    v.chars().next().unwrap().to_ascii_uppercase()
                } else {
                    v.chars().next().unwrap()
                }
            })
    };

    for ch in text.chars() {
        let ch = match ch {
            '\n' | '\r' | '\t' => ' ',
            _ => ch,
        };
        let typo_roll: u32 = rng().random_range(0..1000);
        if typo_roll < 25 {
            if let Some(wrong) = find_nearby(ch) {
                element.type_str(&wrong.to_string()).await?;
                human_pause(160, 420).await;
                element.press_key("Backspace").await?;
                human_pause(80, 200).await;
            }
        }
        let delay = rng().random_range(45..=130);
        element.type_str(&ch.to_string()).await?;
        sleep(Duration::from_millis(delay)).await;
        if rng().random_range(0..100) < 8 {
            human_pause(120, 320).await;
        }
    }
    Ok(())
}

/// Click an element with realistic mouse trajectory + a non-zero
/// down/up delay. chromiumoxide's default .click() teleports.
pub async fn human_click(page: &chromiumoxide::Page, element: &Element) -> Result<()> {
    // chromiumoxide 0.7 doesn't expose a simple `move_mouse` — use CDP directly.
    // bounding_box() returns Result<BoundingBox> directly in 0.7.
    let bb = element.bounding_box().await?;
    let target_x = bb.x + bb.width / 2.0;
    let target_y = bb.y + bb.height / 2.0;

    // A couple of intermediate waypoints with jitter
    let (vw, vh) = (1440.0, 900.0);
    let current_x = vw / 2.0;
    let current_y = vh / 2.0;
    let steps = 3u32;
    for i in 1..=steps {
        let t = i as f64 / (steps + 1) as f64;
        let jx: f64 = rng().random_range(-12..=12) as f64;
        let jy: f64 = rng().random_range(-12..=12) as f64;
        let x = current_x + (target_x - current_x) * t + jx;
        let y = current_y + (target_y - current_y) * t + jy;
        page.execute(
            chromiumoxide::cdp::browser_protocol::input::DispatchMouseEventParams::builder()
                .r#type(chromiumoxide::cdp::browser_protocol::input::DispatchMouseEventType::MouseMoved)
                .x(x)
                .y(y)
                .build()
                .unwrap(),
        )
        .await?;
        human_pause(20, 80).await;
    }

    // Down-wait-up for a realistic click duration
    page.execute(
        chromiumoxide::cdp::browser_protocol::input::DispatchMouseEventParams::builder()
            .r#type(chromiumoxide::cdp::browser_protocol::input::DispatchMouseEventType::MousePressed)
            .x(target_x)
            .y(target_y)
            .button(chromiumoxide::cdp::browser_protocol::input::MouseButton::Left)
            .click_count(1)
            .build()
            .unwrap(),
    )
    .await?;
    human_pause(40, 110).await;
    page.execute(
        chromiumoxide::cdp::browser_protocol::input::DispatchMouseEventParams::builder()
            .r#type(chromiumoxide::cdp::browser_protocol::input::DispatchMouseEventType::MouseReleased)
            .x(target_x)
            .y(target_y)
            .button(chromiumoxide::cdp::browser_protocol::input::MouseButton::Left)
            .click_count(1)
            .build()
            .unwrap(),
    )
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::sanitize_for_typing;

    #[test]
    fn ascii_passes_through_unchanged() {
        let (out, changed) = sanitize_for_typing("hello, world! (2+2)=4");
        assert_eq!(out, "hello, world! (2+2)=4");
        assert!(!changed);
    }

    #[test]
    fn em_dash_and_en_dash_become_hyphen() {
        let (out, changed) = sanitize_for_typing("A \u{2014} B \u{2013} C");
        assert_eq!(out, "A - B - C");
        assert!(changed);
    }

    #[test]
    fn curly_quotes_become_straight() {
        let (out, changed) = sanitize_for_typing("\u{201C}hi\u{201D} he said, \u{2018}ok\u{2019}.");
        assert_eq!(out, "\"hi\" he said, 'ok'.");
        assert!(changed);
    }

    #[test]
    fn ellipsis_expands() {
        let (out, changed) = sanitize_for_typing("wait\u{2026}");
        assert_eq!(out, "wait...");
        assert!(changed);
    }

    #[test]
    fn nbsp_becomes_space() {
        let (out, changed) = sanitize_for_typing("a\u{00A0}b");
        assert_eq!(out, "a b");
        assert!(changed);
    }

    #[test]
    fn mixed_realistic_prompt() {
        let input = "'Attention Is All You Need' \u{2014} Vaswani et al. (2017) \u{2013} one paragraph\u{2026}";
        let (out, changed) = sanitize_for_typing(input);
        assert_eq!(out, "'Attention Is All You Need' - Vaswani et al. (2017) - one paragraph...");
        assert!(changed);
    }
}
