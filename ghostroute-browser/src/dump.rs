//! Rich DOM-shape capture for new-provider bootstrapping (S7 Path A).
//!
//! Returns a JSON blob with: url, title, body innerText sample, top tag/class/
//! data-attribute counts, the top 5 "candidate conversation root" subtrees by
//! repeating-sibling-pattern score, and the top candidate's outerHTML.
//!
//! Receivers (Claude, codex, the human reading the dump) use this to author
//! provider-specific `wait_script` / `extract_script` without a live debugging
//! round-trip.

use anyhow::Result;
use chromiumoxide::Page;
use serde_json::Value;

const DUMP_DOM_SCRIPT: &str = r#"
(() => {
    const truncate = (s, n) => (s && s.length > n) ? s.slice(0, n) + `...[+${s.length - n} chars]` : (s || '');

    // Tag-name histogram across the document.
    const tagCounts = {};
    document.querySelectorAll('*').forEach(e => {
        const t = (e.tagName || '').toLowerCase();
        tagCounts[t] = (tagCounts[t] || 0) + 1;
    });
    const topTags = Object.entries(tagCounts).sort((a, b) => b[1] - a[1]).slice(0, 25);

    // Class-name histogram.
    const classCounts = {};
    document.querySelectorAll('[class]').forEach(e => {
        (e.getAttribute('class') || '').split(/\s+/).filter(Boolean).forEach(c => {
            classCounts[c] = (classCounts[c] || 0) + 1;
        });
    });
    const topClasses = Object.entries(classCounts).sort((a, b) => b[1] - a[1]).slice(0, 30);

    // data-* attribute name histogram.
    const dataAttrCounts = {};
    document.querySelectorAll('*').forEach(e => {
        for (const attr of e.attributes || []) {
            if (attr.name && attr.name.startsWith('data-')) {
                dataAttrCounts[attr.name] = (dataAttrCounts[attr.name] || 0) + 1;
            }
        }
    });
    const topDataAttrs = Object.entries(dataAttrCounts).sort((a, b) => b[1] - a[1]).slice(0, 25);

    // Custom (hyphenated) element names — important for sites that use
    // Lit/Stencil/Angular custom elements (Gemini did this for years).
    const customEls = Object.entries(tagCounts)
        .filter(([t]) => t.includes('-')).sort((a, b) => b[1] - a[1]).slice(0, 20);

    // Heuristic A: candidate conversation roots. Walk all elements; for each
    // with >=4 children, find its longest run of sibling elements with the
    // same tag + first-class signature. Score = run_length * depth.
    const candidates = [];
    Array.from(document.querySelectorAll('*')).forEach(parent => {
        const kids = Array.from(parent.children || []);
        if (kids.length < 4) return;
        const sig = (el) => {
            const t = (el.tagName || '').toLowerCase();
            const c = (el.getAttribute('class') || '').split(/\s+/)[0] || '';
            return `${t}.${c}`;
        };
        let runLen = 1, runSig = sig(kids[0]), bestRun = 1, bestSig = runSig;
        for (let i = 1; i < kids.length; i++) {
            const s = sig(kids[i]);
            if (s === runSig) { runLen++; }
            else { if (runLen > bestRun) { bestRun = runLen; bestSig = runSig; } runLen = 1; runSig = s; }
        }
        if (runLen > bestRun) { bestRun = runLen; bestSig = runSig; }
        if (bestRun < 2) return;

        let depth = 0; let p = parent;
        while (p && p.parentElement) { depth++; p = p.parentElement; }

        const parentSel = (() => {
            const t = (parent.tagName || '').toLowerCase();
            const id = parent.id ? `#${parent.id}` : '';
            const cls = (parent.getAttribute('class') || '').split(/\s+/).filter(Boolean).slice(0, 3).map(c => `.${c}`).join('');
            return `${t}${id}${cls}`;
        })();

        candidates.push({
            parent_selector: parentSel,
            child_signature: bestSig,
            run_length: bestRun,
            total_kids: kids.length,
            depth,
            score: bestRun * Math.max(1, depth),
        });
    });
    candidates.sort((a, b) => b.score - a.score);
    const topCandidates = candidates.slice(0, 5);

    let topOuterHtml = null;
    if (topCandidates.length > 0) {
        try {
            const node = document.querySelector(topCandidates[0].parent_selector);
            if (node) topOuterHtml = truncate(node.outerHTML || '', 12000);
        } catch (_) {}
    }

    const bodySample = truncate((document.body && document.body.innerText) || '', 1500);

    return {
        url: location.href,
        title: document.title,
        body_sample: bodySample,
        top_tags: topTags,
        top_classes: topClasses,
        top_data_attrs: topDataAttrs,
        top_custom_elements: customEls,
        candidate_roots: topCandidates,
        top_root_outer_html: topOuterHtml,
    };
})();
"#;

/// Run the dump-DOM JS against the current page.
pub async fn capture_dump_dom(page: &Page) -> Result<Value> {
    let result: Value = page
        .evaluate(DUMP_DOM_SCRIPT)
        .await?
        .into_value()
        .map_err(|e| anyhow::anyhow!("dump-dom result not JSON-serialisable: {e}"))?;
    Ok(result)
}
