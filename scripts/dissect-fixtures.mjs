// Offline fixture dissector.
//
// Loads each captured Perplexity HTML fixture and dumps the internal
// structure we need for Task 10 (selectors.js):
//   - source cards inside [class*="group/search-side-content"]
//   - inline citation anchors inside div[id^="markdown-content-"]
//   - Deep Research "Completed N steps" button + its following content
//
// Runs offline (cheerio + node:fs). No browser. No network.
//
// Usage:
//   node scripts/dissect-fixtures.mjs
//   node scripts/dissect-fixtures.mjs auto-web    # just one fixture
//
// Output: human-readable report per fixture to stdout.

import fs from 'node:fs';
import path from 'node:path';
import * as cheerio from 'cheerio';

const FIXTURE_DIR = path.resolve('providers/perplexity/__fixtures__');
const filter = process.argv[2] || null;

const fixtures = fs
  .readdirSync(FIXTURE_DIR)
  .filter((f) => f.endsWith('.html'))
  .filter((f) => !filter || f.includes(filter));

if (fixtures.length === 0) {
  console.error(`No fixtures matched filter=${filter}`);
  process.exit(1);
}

function briefAttrs(el) {
  const attrs = el.attribs || {};
  const keys = Object.keys(attrs);
  const brief = {};
  for (const k of keys) {
    if (k === 'class') brief[k] = attrs[k].slice(0, 120);
    else brief[k] = attrs[k].slice(0, 80);
  }
  return brief;
}

function tagShort(el) {
  const a = briefAttrs(el);
  const bits = [el.name];
  if (a.id) bits.push(`#${a.id}`);
  if (a.role) bits.push(`role=${a.role}`);
  if (a['data-testid']) bits.push(`testid=${a['data-testid']}`);
  if (a.href) bits.push(`href=${a.href.slice(0, 60)}`);
  if (a.class) bits.push(`class="${a.class.slice(0, 80)}"`);
  return bits.join(' ');
}

for (const fname of fixtures) {
  const full = path.join(FIXTURE_DIR, fname);
  const html = fs.readFileSync(full, 'utf8');
  const $ = cheerio.load(html);

  console.log('\n' + '='.repeat(72));
  console.log(`FIXTURE: ${fname}  (${(html.length / 1024).toFixed(1)} KB)`);
  console.log('='.repeat(72));

  // -------- 1) answer container presence
  const markdown = $('div[id^="markdown-content-"]');
  console.log(`\n[1] answer containers (div[id^="markdown-content-"]): ${markdown.length}`);
  markdown.slice(0, 3).each((i, el) => {
    const id = $(el).attr('id');
    const textLen = $(el).text().trim().length;
    console.log(`    #${i}: id=${id}  textLen=${textLen}`);
  });

  // -------- 2) citation patterns inside the LAST answer
  const lastAnswer = markdown.last();
  if (lastAnswer.length) {
    const answerText = lastAnswer.text();
    const textCitations = answerText.match(/\[\d+\]/g) || [];
    const anchors = lastAnswer.find('a');
    const sups = lastAnswer.find('sup');
    const numericAnchors = [];
    const otherAnchors = [];
    anchors.each((_, el) => {
      const $el = $(el);
      const txt = $el.text().trim();
      const href = $el.attr('href') || '';
      const entry = { text: txt.slice(0, 40), href: href.slice(0, 80), attrs: Object.keys(el.attribs || {}) };
      if (/^\[?\d+\]?$/.test(txt)) numericAnchors.push(entry);
      else otherAnchors.push(entry);
    });
    console.log(`\n[2] citation markers inside answer text`);
    console.log(`    plain-text [N] matches: ${textCitations.length} (${textCitations.slice(0, 8).join(', ')})`);
    console.log(`    <a> total=${anchors.length}  numeric-text=${numericAnchors.length}  other-text=${otherAnchors.length}`);
    console.log(`    <sup> total=${sups.length}`);
    // Also search the WHOLE document for any citation-looking elements we might have missed
    const pageSups = $('sup');
    const pageNumericAnchors = [];
    $('a').each((_, el) => {
      const t = $(el).text().trim();
      if (/^\[?\d+\]?$/.test(t)) pageNumericAnchors.push({ text: t, href: $(el).attr('href') || '' });
    });
    console.log(`    (whole page) <sup> total=${pageSups.length}, numeric-text <a> total=${pageNumericAnchors.length}`);
    pageSups.slice(0, 3).each((i, el) => {
      const $s = $(el);
      console.log(`      <sup> #${i}: text="${$s.text().trim().slice(0, 30)}"  parentTag=${el.parent?.name}`);
    });
    pageNumericAnchors.slice(0, 3).forEach((a, i) =>
      console.log(`      <a> #${i}: text="${a.text}"  href=${a.href.slice(0, 50)}`)
    );
    // Look for "Academic" / "Finance" / etc in the HTML (for focus-filter question)
    const topicKeywords = ['Academic', 'Finance', 'Health', 'Patents', 'Discover'];
    const foundTopics = topicKeywords.filter((t) => lastAnswer.closest('html').length && $.html().includes(t));
    console.log(`    topic keywords present in page: [${foundTopics.join(', ')}]`);
    numericAnchors.slice(0, 3).forEach((a, i) =>
      console.log(`      <a> #${i}: text="${a.text}"  href=${a.href}  attrs=[${a.attrs.join(',')}]`)
    );
    sups.slice(0, 3).each((i, el) => {
      const $s = $(el);
      console.log(`      <sup> #${i}: text="${$s.text().trim().slice(0, 30)}"  inner=${$s.html()?.slice(0, 80)}`);
    });
    // Show answer text snippet around first citation
    if (textCitations.length) {
      const firstIdx = answerText.indexOf(textCitations[0]);
      console.log(`    context around first [N]: "...${answerText.slice(Math.max(0, firstIdx - 30), firstIdx + 30)}..."`);
    }
  }

  // -------- 3) sources overlay
  const sourcesOverlay = $('[class*="group/search-side-content"]');
  console.log(`\n[3] sources overlay [class*="group/search-side-content"]: ${sourcesOverlay.length} matches`);
  sourcesOverlay.slice(0, 2).each((i, el) => {
    const $el = $(el);
    const directChildren = $el.children();
    console.log(`    overlay #${i}: tag=${el.name} directChildren=${directChildren.length}`);
    console.log(`      attrs: ${JSON.stringify(briefAttrs(el))}`);

    // Look at the deepest level containers — cards usually live 1-3 levels deep
    const linkDescendants = $el.find('a[href]');
    console.log(`      <a href> descendants: ${linkDescendants.length}`);
    // Full dissection of FIRST card to find title/snippet selectors
    if (linkDescendants.length > 0) {
      const $first = $(linkDescendants[0]);
      console.log(`\n      >>> FIRST CARD FULL STRUCTURE (depth 3) <<<`);
      function walk($node, depth, maxDepth) {
        if (depth > maxDepth) return;
        const pad = '        '.repeat(depth);
        $node.children().each((_, c) => {
          console.log(`${pad}${tagShort(c)}`);
          const text = $(c).clone().children().remove().end().text().trim();
          if (text) console.log(`${pad}  text: "${text.slice(0, 60)}"`);
          walk($(c), depth + 1, maxDepth);
        });
      }
      walk($first, 0, 3);
      console.log();
    }
    // Also summarize just the hrefs
    linkDescendants.slice(0, 3).each((j, link) => {
      const $link = $(link);
      const href = $link.attr('href') || '';
      console.log(`        summary #${j}: href=${href.slice(0, 60)}`);
    });
  });

  // -------- 4) Deep Research — "Completed N steps" disclosure
  const completedBtns = $('button').filter((_, el) => /completed\s+\d+\s+steps?/i.test($(el).text()));
  console.log(`\n[4] "Completed N steps" disclosure buttons: ${completedBtns.length}`);
  completedBtns.slice(0, 2).each((i, el) => {
    const $btn = $(el);
    const ariaExpanded = $btn.attr('aria-expanded');
    const ariaControls = $btn.attr('aria-controls');
    console.log(`    btn #${i}: text="${$btn.text().trim().slice(0, 60)}"  aria-expanded=${ariaExpanded}  aria-controls=${ariaControls}`);

    // Look at following siblings — the steps list might be the next div
    const next = $btn.next();
    if (next.length) {
      console.log(`      next sibling: ${tagShort(next[0])}`);
      console.log(`        direct children: ${next.children().length}`);
      next.children().slice(0, 5).each((j, child) => {
        console.log(`          child#${j}: ${tagShort(child)}`);
      });
    }

    // Also check aria-controls target if present
    if (ariaControls) {
      const controlled = $(`#${ariaControls}`);
      if (controlled.length) {
        console.log(`      aria-controls target #${ariaControls}: ${tagShort(controlled[0])}`);
      }
    }

    // Walk up to parent and look for sibling panels
    const parent = $btn.parent();
    console.log(`      parent: ${tagShort(parent[0] || { name: 'null' })}`);
  });

  // -------- 5) Sidebar / nav items (for focus-filter question)
  const navLinks = $('nav a, aside a, [role="navigation"] a');
  console.log(`\n[5] sidebar/nav anchor texts (for focus-filter question): ${navLinks.length} links`);
  const seen = new Set();
  navLinks.each((_, el) => {
    const t = $(el).text().trim().replace(/\s+/g, ' ').slice(0, 40);
    const h = ($(el).attr('href') || '').slice(0, 60);
    const key = `${t}|${h}`;
    if (seen.has(key)) return;
    seen.add(key);
    console.log(`    "${t.padEnd(30)}" → ${h}`);
  });
}

console.log('\n');
