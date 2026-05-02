use chromiumoxide::Page;

/// Dump the DOM state at the moment of locator failure so a stale selector
/// gives us the *actual* current shape of input candidates instead of a
/// blank 12s timeout. Costs one CDP round-trip; only fires on bail.
pub async fn probe_dom_state(page: &Page) -> Option<String> {
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

/// Dump the response-container state at stability-wait timeout. Surfaces the
/// shape that matters for diagnosing *why* the wait failed: how many turns
/// rendered, did the assistant turn arrive at all, what's the gap between
/// `innerText` and `textContent` lengths (closed `<details>` widen this gap),
/// and whether bullets / list items are present in the DOM but missing from
/// the captured text.
pub async fn probe_response_state(page: &Page, selector: &str) -> Option<String> {
    // Selector is a static const in config/mod.rs; round-trip through JSON
    // so any future quoting changes there don't break the JS payload.
    let selector_literal = serde_json::to_string(selector).ok()?;
    let js = format!(
        r#"(() => {{
          const els = Array.from(document.querySelectorAll({selector_literal}));
          const describe = (e, i) => ({{
            index: i,
            id: e.id || null,
            visible: !!(e.offsetWidth || e.offsetHeight),
            innerTextLen: (e.innerText || '').length,
            textContentLen: (e.textContent || '').length,
            paragraphCount: e.querySelectorAll('p').length,
            listItemCount: e.querySelectorAll('li').length,
            detailsOpen: e.querySelectorAll('details[open]').length,
            detailsClosed: e.querySelectorAll('details:not([open])').length,
            preview: (e.innerText || '').slice(0, 200),
          }});
          return JSON.stringify({{
            url: location.href,
            title: document.title,
            selector: {selector_literal},
            responseCount: els.length,
            responses: els.map(describe),
          }}, null, 2);
        }})()"#
    );
    let value = page.evaluate(js).await.ok()?;
    value.into_value::<String>().ok()
}
