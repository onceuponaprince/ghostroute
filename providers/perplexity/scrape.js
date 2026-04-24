import { chromium } from 'playwright';
import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import { PerplexityAuthError, PerplexityScrapeError } from './errors.js';
import { SELECTORS, FOCUS_URLS } from './selectors.js';
import { humanClick, humanPause } from './human.js';

const COOKIE_PATH = path.join(os.homedir(), '.claude', 'cookie-configs', 'perplexity.ai-cookies.json');

export async function launchAndNavigate({ focus = 'web', threadId } = {}) {
  if (!fs.existsSync(COOKIE_PATH)) {
    throw new PerplexityAuthError();
  }
  const cookies = JSON.parse(fs.readFileSync(COOKIE_PATH, 'utf8'));

  // Two layers of automation detection to defeat:
  //
  // 1) Cloudflare challenge — gets tripped by the `headless` flag itself.
  //    Running headed with stock Playwright passes it. Tested patchright
  //    (stealth-patched Playwright) + persistent context + Chrome channel,
  //    all in headless — none bypassed. Only `headless: false` works.
  //    For headless servers, run under Xvfb.
  //
  // 2) Perplexity's own Pro-feature gating — serves a stub Model menu (only
  //    "Sonar") to detected automation. Bypassed by disabling the blink
  //    automation flag, suppressing the --enable-automation default arg,
  //    and overriding navigator.webdriver at JS level. Without these, the
  //    full model list (7 models) is hidden.
  const browser = await chromium.launch({
    headless: false,
    args: ['--disable-blink-features=AutomationControlled'],
    ignoreDefaultArgs: ['--enable-automation'],
  });
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  await context.addCookies(cookies);
  await context.addInitScript(() => {
    Object.defineProperty(navigator, 'webdriver', { get: () => undefined });
    window.chrome = window.chrome || { runtime: {} };
    Object.defineProperty(navigator, 'languages', { get: () => ['en-US', 'en'] });
  });
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

// Model enum → menu label. Labels use prefix-matching so UI version bumps
// (e.g. "Claude Sonnet 4.6" → "Claude Sonnet 5.0") still work.
const MODEL_LABELS = {
  best: 'Best',
  sonar: 'Sonar',
  gpt: 'GPT',
  gemini: 'Gemini',
  claude: 'Claude',
  kimi: 'Kimi',
  nemotron: 'Nemotron',
};

// Tool enum → + menu label.
const TOOL_LABELS = {
  'deep-research': 'Deep research',
};

export async function selectModel(page, model = 'best') {
  const label = MODEL_LABELS[model];
  if (!label) {
    throw new PerplexityScrapeError('select-model', 'unknown model', `model=${model}`);
  }
  try {
    await humanClick(page, page.locator(SELECTORS.modelButton));
    await humanPause(page, 400, 900);
    await humanClick(page, page.locator(SELECTORS.menuRadio(label)));
    await humanPause(page, 200, 500);
  } catch {
    throw new PerplexityScrapeError('select-model', SELECTORS.modelButton, await page.content());
  }
}

export async function selectTool(page, tool) {
  if (!tool) return; // no tool selected → leave the menu closed
  const label = TOOL_LABELS[tool];
  if (!label) {
    throw new PerplexityScrapeError('select-tool', 'unknown tool', `tool=${tool}`);
  }
  try {
    await humanClick(page, page.locator(SELECTORS.toolsButton));
    await humanPause(page, 400, 900);
    await humanClick(page, page.locator(SELECTORS.menuRadio(label)));
    await humanPause(page, 200, 500);
  } catch {
    throw new PerplexityScrapeError('select-tool', SELECTORS.toolsButton, await page.content());
  }
}
