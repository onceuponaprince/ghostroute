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

  // headless: false is load-bearing. Cloudflare detects the headless flag
  // directly and serves a verify-human interstitial that returns ~31 KB of
  // challenge HTML instead of the ~360 KB app payload. Tested with stock
  // Playwright headed, patchright (stealth-patched) headless+Chrome-channel,
  // and persistent context headless — only headed passes the gate.
  // For headless servers, run under Xvfb.
  const browser = await chromium.launch({ headless: false });
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  await context.addCookies(cookies);
  const page = await context.newPage();

  // Threading wins over focus — continuing a thread navigates to its URL directly.
  const entryPath = FOCUS_URLS[focus] ?? '/';
  const url = threadId
    ? `https://www.perplexity.ai/search/${encodeURIComponent(threadId)}`
    : `https://www.perplexity.ai${entryPath}`;

  await page.goto(url, { waitUntil: 'domcontentloaded' });

  // If we hit a Cloudflare challenge the URL carries a __cf_chl_rt_tk token.
  // Wait briefly for the challenge to resolve; if it hasn't after 6s, the
  // gate probably kicked us to a login wall or blocked outright.
  if (page.url().includes('__cf_chl_rt_tk')) {
    await page.waitForURL((u) => !u.toString().includes('__cf_chl_rt_tk'), { timeout: 15_000 }).catch(() => {});
  }

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
