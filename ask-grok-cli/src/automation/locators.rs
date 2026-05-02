use anyhow::{bail, Result};
use chromiumoxide::{Element, Page};
use std::time::{Duration, Instant};
use tokio::time::sleep;

use super::probe::probe_dom_state;

pub enum Position {
    First,
    Last,
}

pub async fn find_visible_locator(
    page: &Page,
    selector: &str,
    timeout_ms: f64,
    step_name: &str,
    position: Position,
) -> Result<Element> {
    let started = Instant::now();
    let timeout = Duration::from_millis(timeout_ms as u64);

    loop {
        let elements = page.find_elements(selector).await.unwrap_or_default();
        if !elements.is_empty() {
            let idx = match position {
                Position::First => 0,
                Position::Last => elements.len() - 1,
            };
            if let Some(element) = elements.into_iter().nth(idx) {
                return Ok(element);
            }
        }

        if started.elapsed() >= timeout {
            if let Some(probe) = probe_dom_state(page).await {
                eprintln!(
                    "[probe] DOM state at {step_name} failure (selector='{selector}'):\n{probe}"
                );
            } else {
                eprintln!("[probe] DOM probe itself failed — page may be unresponsive");
            }
            bail!(
                "{}: selector '{}' did not become visible within {}ms",
                step_name,
                selector,
                timeout_ms
            );
        }

        sleep(Duration::from_millis(200)).await;
    }
}
