use anyhow::Result;
use chromiumoxide::Element;
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

pub async fn human_type_with_typos(chat_input: &Element, text: &str) -> Result<()> {
    let typos = ['s', 'd', 'f', 'g', 'x', 'c'];
    let mut rng = rand::rng();

    for (idx, c) in text.chars().enumerate() {
        // CDP's single-char keymap has no entry for newlines/tabs; pressing Enter
        // would submit the message. Substitute with space so content still flows.
        let c = if c == '\n' || c == '\r' || c == '\t' { ' ' } else { c };

        if idx > 0 {
            let delay = rng.random_range(30..150);
            sleep(Duration::from_millis(delay)).await;
        }

        let mistake_roll = rng.random_range(1..=100);
        if mistake_roll <= 5 {
            let bad_char = typos[rng.random_range(0..typos.len())];
            chat_input.type_str(&bad_char.to_string()).await?;
            sleep(Duration::from_millis(250)).await;
            chat_input.press_key("Backspace").await?;
            sleep(Duration::from_millis(100)).await;
        }

        chat_input.type_str(&c.to_string()).await?;
    }

    Ok(())
}
