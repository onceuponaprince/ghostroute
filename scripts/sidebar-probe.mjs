// Sidebar / focus-filter probe.
//
// Live navigate to perplexity.ai with cookies and dump the COMPLETE
// sidebar / nav / topic structure so we can resolve the focus-filter
// question (did web/academic/writing get replaced by a Spaces or
// Discover-topics pattern, and is it URL-routable?).
//
// Usage:
//   node scripts/sidebar-probe.mjs
//
// Output:
//   - All <a href> elements grouped by href-pattern (plain /, /search/*, /spaces, /discover/*, etc.)
//   - Any element carrying text matching "Academic | Finance | Health | Patents | Discover | Writing | Web"
//   - Screenshot to probe-sidebar.png

import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { chromium } from 'playwright';

const cookiePath = path.join(os.homedir(), '.claude', 'cookie-configs', 'perplexity.ai-cookies.json');
const cookies = JSON.parse(fs.readFileSync(cookiePath, 'utf8'));

const browser = await chromium.launch({ headless: false });
const context = await browser.newContext({
  viewport: { width: 1440, height: 900 },
  userAgent: 'Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36',
});
await context.addCookies(cookies);

const page = await context.newPage();
await page.goto('https://www.perplexity.ai/', { waitUntil: 'domcontentloaded', timeout: 30_000 });

// Let dynamic sidebar content load
await page.waitForTimeout(6_000);

const keywords = ['Academic', 'Finance', 'Health', 'Patents', 'Discover', 'Writing', 'Web', 'Social', 'Spaces', 'Library'];

const dump = await page.evaluate((keywords) => {
  // Collect every <a href> and group by href prefix
  const byPrefix = new Map();
  document.querySelectorAll('a[href]').forEach((a) => {
    const href = a.getAttribute('href') || '';
    const text = (a.textContent || '').trim().replace(/\s+/g, ' ').slice(0, 80);
    const aria = a.getAttribute('aria-label') || '';
    const prefix = href.match(/^\/[^/?#]+/)?.[0] || '(root)';
    const list = byPrefix.get(prefix) || [];
    list.push({ href: href.slice(0, 120), text, aria: aria.slice(0, 60) });
    byPrefix.set(prefix, list);
  });

  // Dedupe each list
  const prefixes = {};
  for (const [k, v] of byPrefix.entries()) {
    const seen = new Set();
    const unique = v.filter((e) => {
      const key = `${e.href}|${e.text}`;
      if (seen.has(key)) return false;
      seen.add(key);
      return true;
    });
    prefixes[k] = unique.slice(0, 10);
  }

  // Hunt for elements containing each keyword
  const found = {};
  for (const kw of keywords) {
    const hits = [];
    document.querySelectorAll('*').forEach((el) => {
      if (el.children.length > 0) return; // leaf nodes only
      const t = (el.textContent || '').trim();
      if (t && t.length < 60 && new RegExp(`\\b${kw}\\b`, 'i').test(t)) {
        // Walk up to find nearest link or button
        let up = el;
        let container = null;
        for (let i = 0; i < 6 && up; i++) {
          if (up.tagName === 'A' || up.tagName === 'BUTTON') {
            container = up;
            break;
          }
          up = up.parentElement;
        }
        hits.push({
          text: t.slice(0, 60),
          tag: el.tagName,
          containerTag: container?.tagName || '(none)',
          containerHref: container?.getAttribute?.('href') || '',
          containerAria: container?.getAttribute?.('aria-label') || '',
        });
      }
    });
    if (hits.length) found[kw] = hits.slice(0, 5);
  }

  return { prefixes, found, bodyPreview: (document.body.innerText || '').slice(0, 500) };
}, keywords);

console.log('='.repeat(72));
console.log('BY HREF PREFIX');
console.log('='.repeat(72));
for (const [prefix, entries] of Object.entries(dump.prefixes)) {
  console.log(`\n${prefix}  (${entries.length} unique entries shown, up to 10)`);
  for (const e of entries) {
    const label = e.text || e.aria || '(no text)';
    console.log(`  "${label.padEnd(40).slice(0, 40)}" → ${e.href}`);
  }
}

console.log('\n' + '='.repeat(72));
console.log('KEYWORD HITS');
console.log('='.repeat(72));
for (const [kw, hits] of Object.entries(dump.found)) {
  console.log(`\n[${kw}]`);
  for (const h of hits) {
    console.log(`  text="${h.text}" tag=${h.tag} container=${h.containerTag} href=${h.containerHref} aria=${h.containerAria}`);
  }
}

await page.screenshot({ path: 'probe-sidebar.png', fullPage: false });
console.log('\n[probe] screenshot: probe-sidebar.png');

await browser.close();
