// Probe the Model menu using humanized interaction — hypothesis is that
// robotic teleport-clicks trigger automation detection and Perplexity serves
// a stripped-down menu (only Sonar) versus the full list a real user sees.

import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { chromium } from 'playwright';
import { humanClick, humanPause, randomDelay } from '../providers/perplexity/human.js';

const cookies = JSON.parse(
  fs.readFileSync(path.join(os.homedir(), '.claude', 'cookie-configs', 'perplexity.ai-cookies.json'), 'utf8')
);

const browser = await chromium.launch({ headless: false });
const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
await context.addCookies(cookies);
const page = await context.newPage();

await page.goto('https://www.perplexity.ai/', { waitUntil: 'domcontentloaded' });
await humanPause(page, 3500, 5500);  // let the page settle

// A few exploratory mouse wiggles before the meaningful click —
// suggests a user who's actually looking at the page.
for (let i = 0; i < 3; i++) {
  await page.mouse.move(randomDelay(200, 1200), randomDelay(200, 700), { steps: 8 });
  await humanPause(page, 150, 400);
}

console.error('[probe] humanClick on Model button');
await humanClick(page, page.locator('button[aria-label="Model"]'));
await humanPause(page, 1500, 2500);  // generous wait for Radix portal + any lazy content

// Sometimes Radix menus only fully hydrate when you hover items / scroll.
// Do a small scroll inside the menu to trigger any virtualized children.
const menuId = await page.locator('button[aria-label="Model"]').first().getAttribute('aria-controls');
if (menuId) {
  // Try scrolling the menu content
  await page.evaluate((id) => {
    const menu = document.getElementById(id);
    if (!menu) return;
    const scrollers = menu.querySelectorAll('[data-radix-scroll-area-viewport]');
    for (const s of scrollers) s.scrollTop = 200;
  }, menuId);
  await humanPause(page, 400, 800);
}

// Dump the full menu contents (visible + off-screen)
const result = await page.evaluate((id) => {
  const menu = id ? document.getElementById(id) : null;
  if (!menu) return { error: `menu #${id} not found` };
  const items = [];
  menu.querySelectorAll('[role="menuitem"], [role="menuitemradio"], [role="option"]').forEach((el) => {
    const rect = el.getBoundingClientRect();
    items.push({
      role: el.getAttribute('role'),
      text: (el.textContent || '').trim().replace(/\s+/g, ' ').slice(0, 80),
      ariaChecked: el.getAttribute('aria-checked') || '',
      dataState: el.getAttribute('data-state') || '',
      visible: rect.width > 0 && rect.height > 0,
    });
  });
  return { items, menuHtmlLen: menu.outerHTML.length };
}, menuId);

console.log('=== Model menu (humanized) ===');
if (result.error) {
  console.log('ERROR:', result.error);
} else {
  console.log(`menuHtml bytes: ${result.menuHtmlLen}`);
  console.log(`total menuitem-like elements: ${result.items.length}`);
  for (const it of result.items) {
    const flag = it.visible ? 'V' : ' ';
    console.log(`  [${flag}] role=${it.role.padEnd(16)} checked=${it.ariaChecked.padEnd(5)} "${it.text}"`);
  }
}

await page.screenshot({ path: 'probe-model-menu-human.png', fullPage: false });
console.error('[probe] screenshot: probe-model-menu-human.png');
console.error('[probe] browser stays open 20s for inspection');
await humanPause(page, 20_000, 20_000);
await browser.close();
