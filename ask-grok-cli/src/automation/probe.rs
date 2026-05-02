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
