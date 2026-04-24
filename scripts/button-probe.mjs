// Button probe — dump every visible button on perplexity.ai.
// Used to diagnose where Pro / Reasoning / Deep Research mode selectors actually live.
//
// Usage:
//   node scripts/button-probe.mjs                 # homepage
//   node scripts/button-probe.mjs "some prompt"   # submit a prompt then dump buttons

import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { chromium } from 'playwright';

const promptArg = process.argv[2] || null;

const cookies = JSON.parse(
  fs.readFileSync(path.join(os.homedir(), '.claude', 'cookie-configs', 'perplexity.ai-cookies.json'), 'utf8')
);

const browser = await chromium.launch({ headless: false });
const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
await context.addCookies(cookies);
const page = await context.newPage();

await page.goto('https://www.perplexity.ai/', { waitUntil: 'domcontentloaded' });
await page.waitForTimeout(4000);

if (promptArg) {
  console.error(`[probe] typing prompt: ${promptArg}`);
  await page.locator('div[contenteditable="true"][role="textbox"]').first().click();
  await page.keyboard.type(promptArg, { delay: 15 });
  await page.waitForTimeout(1500);
}

// Click the Model button and dump menu contents
console.error('[probe] clicking Model button');
await page.locator('button[aria-label="Model"]').first().click().catch((e) => console.error('Model click failed:', e.message));
await page.waitForTimeout(1200);

// Dump everything with a role attribute, plus anything inside a popover/dialog
const menuStuff = await page.evaluate(() => {
  const results = [];

  // Anything with a role
  for (const el of document.querySelectorAll('[role]')) {
    const r = el.getAttribute('role');
    if (['menuitem', 'option', 'menuitemradio', 'menuitemcheckbox', 'dialog', 'listbox'].includes(r)) {
      const rect = el.getBoundingClientRect();
      if (rect.width === 0 || rect.height === 0) continue;
      results.push({
        tag: el.tagName.toLowerCase(),
        role: r,
        text: (el.textContent || '').trim().replace(/\s+/g, ' ').slice(0, 80),
        dataTestid: el.getAttribute('data-testid') || '',
      });
    }
  }

  // Any clickable item inside a popover or radix dropdown — look for data-radix-* attrs
  const radixContent = document.querySelector('[data-radix-popper-content-wrapper], [data-state="open"][role="menu"], [data-state="open"]');
  if (radixContent) {
    for (const el of radixContent.querySelectorAll('button, a, [role="button"], [data-mode], [data-value]')) {
      const rect = el.getBoundingClientRect();
      if (rect.width === 0 || rect.height === 0) continue;
      results.push({
        kind: 'in-popover',
        tag: el.tagName.toLowerCase(),
        text: (el.textContent || '').trim().replace(/\s+/g, ' ').slice(0, 80),
        ariaLabel: el.getAttribute('aria-label') || '',
        dataValue: el.getAttribute('data-value') || '',
        dataMode: el.getAttribute('data-mode') || '',
        dataTestid: el.getAttribute('data-testid') || '',
      });
    }
  }

  // Snapshot the radix content's outerHTML (truncated)
  const popoverSnippet = radixContent ? radixContent.outerHTML.slice(0, 2000) : '(no radix popper content found)';

  return { results, popoverSnippet };
});

console.log('\n=== Model menu — matched elements ===');
for (const m of menuStuff.results) {
  const bits = [];
  if (m.kind) bits.push(`[${m.kind}]`);
  if (m.tag) bits.push(m.tag);
  if (m.role) bits.push(`role=${m.role}`);
  if (m.text) bits.push(`"${m.text}"`);
  if (m.ariaLabel) bits.push(`aria-label="${m.ariaLabel}"`);
  if (m.dataMode) bits.push(`data-mode=${m.dataMode}`);
  if (m.dataValue) bits.push(`data-value=${m.dataValue}`);
  if (m.dataTestid) bits.push(`testid=${m.dataTestid}`);
  console.log('  ' + bits.join('  '));
}
console.log('\n=== Radix popper content (first 2KB) ===');
console.log(menuStuff.popoverSnippet);

const dump = await page.evaluate(() => {
  const buttons = Array.from(document.querySelectorAll('button'));
  return buttons
    .filter((b) => {
      const rect = b.getBoundingClientRect();
      return rect.width > 0 && rect.height > 0;
    })
    .map((b) => ({
      text: (b.textContent || '').trim().replace(/\s+/g, ' ').slice(0, 80),
      ariaLabel: b.getAttribute('aria-label') || '',
      label: b.getAttribute('label') || '',
      ariaHaspopup: b.getAttribute('aria-haspopup') || '',
      ariaExpanded: b.getAttribute('aria-expanded') || '',
      classes: (b.className || '').slice(0, 80),
    }))
    .filter((b) => b.text || b.ariaLabel || b.label);
});

console.log(`\n=== ${dump.length} visible buttons ===\n`);
for (const b of dump) {
  const bits = [];
  if (b.text) bits.push(`"${b.text}"`);
  if (b.ariaLabel) bits.push(`aria-label="${b.ariaLabel}"`);
  if (b.label) bits.push(`label="${b.label}"`);
  if (b.ariaHaspopup) bits.push(`aria-haspopup="${b.ariaHaspopup}"`);
  if (b.ariaExpanded) bits.push(`aria-expanded="${b.ariaExpanded}"`);
  console.log('  ' + bits.join('  '));
}

await page.screenshot({ path: 'probe-buttons.png', fullPage: false });
console.error('\n[probe] screenshot: probe-buttons.png');
console.error('[probe] browser will stay open for 20s — interact manually if desired');
await page.waitForTimeout(20_000);
await browser.close();
