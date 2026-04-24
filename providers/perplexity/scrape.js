import { chromium } from 'playwright';
import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import { PerplexityAuthError, PerplexityScrapeError } from './errors.js';
import { SELECTORS, FOCUS_URLS } from './selectors.js';

const COOKIE_PATH = path.join(os.homedir(), '.claude', 'cookie-configs', 'perplexity.ai-cookies.json');

export async function launchAndNavigate({ focus = 'web', threadId } = {}) {
  if (!fs.existsSync(COOKIE_PATH)) {
    throw new PerplexityAuthError();
  }
  const cookies = JSON.parse(fs.readFileSync(COOKIE_PATH, 'utf8'));

  const browser = await chromium.launch({ headless: true });
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  await context.addCookies(cookies);
  const page = await context.newPage();

  // Threading wins over focus — continuing a thread navigates to its URL directly.
  const entryPath = FOCUS_URLS[focus] ?? '/';
  const url = threadId
    ? `https://www.perplexity.ai/search/${encodeURIComponent(threadId)}`
    : `https://www.perplexity.ai${entryPath}`;

  await page.goto(url, { waitUntil: 'domcontentloaded' });

  // Fail fast if we landed on a login wall.
  if (SELECTORS.loginWallDetector) {
    const wall = await page.locator(SELECTORS.loginWallDetector).first().isVisible().catch(() => false);
    if (wall) {
      await browser.close();
      throw new PerplexityAuthError();
    }
  }

  return { browser, context, page };
}
