use anyhow::{bail, Result};
use chromiumoxide::{Element, Page};
use std::time::{Duration, Instant};
use tokio::time::sleep;

pub enum Position {
    First,
    Last,
}

/// Dump the DOM state at the moment of locator failure so a stale selector
/// gives us the *actual* current shape of input candidates instead of a
/// blank 12s timeout. Costs one CDP round-trip; only fires on bail.
async fn probe_dom_state(page: &Page) -> Option<String> {
    let js = r#"(() => {
      const editables = Array.from(document.querySelectorAll('[contenteditable="true"]'));
      const textareas = Array.from(document.querySelectorAll('textarea'));
      const inputs = Array.from(document.querySelectorAll('input[type="text"], input:not([type])'));
      const describe = (e) => ({
        tag: e.tagName.toLowerCase(),
        class: e.className && e.className.toString ? e.className.toString() : null,
        id: e.id || null,
        tabindex: e.getAttribute('tabindex'),
        ariaLabel: e.getAttribute('aria-label'),
        role: e.getAttribute('role'),
        placeholder: e.getAttribute('placeholder'),
        dataTestid: e.getAttribute('data-testid'),
        visible: !!(e.offsetWidth || e.offsetHeight),
      });
      return JSON.stringify({
        url: location.href,
        title: document.title,
        bodyTextLen: (document.body && document.body.innerText || '').length,
        contentEditable: editables.map(describe),
        textareas: textareas.map(describe),
        textInputs: inputs.map(describe),
      }, null, 2);
    })()"#;
    let value = page.evaluate(js).await.ok()?;
    value.into_value::<String>().ok()
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
