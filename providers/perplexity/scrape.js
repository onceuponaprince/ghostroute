import { chromium } from 'playwright';
import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import { PerplexityAuthError, PerplexityScrapeError, PerplexityTimeoutError } from './errors.js';
import { SELECTORS, FOCUS_URLS } from './selectors.js';
import { humanClick, humanPause, humanType } from './human.js';

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

const FAST_TIMEOUT_MS = 180_000;    // 3 min end-to-end
const DEEP_TIMEOUT_MS = 1_800_000;  // 30 min end-to-end
const NO_PROGRESS_MS = 300_000;     // 5 min, tripped if DR stops emitting progress

// Adaptive completion detector: once the answer container appears, watch its
// text length. When it stops growing for `stableMs` ms, we call it done.
// Robust against UI changes because it doesn't rely on specific spinner
// class names (those drift constantly).
async function waitForAnswerStable(page, { totalTimeoutMs, stableMs, onProgress }) {
  const start = Date.now();
  let lastLen = 0;
  let lastChange = Date.now();
  let lastPhaseCount = 0;

  // First: wait for the answer container to even appear.
  try {
    await page.waitForSelector(SELECTORS.answerContainer, { timeout: 60_000 });
  } catch {
    throw new PerplexityTimeoutError('answer-not-rendered', 60_000);
  }

  while (Date.now() - start < totalTimeoutMs) {
    const len = await page.locator(SELECTORS.answerContainer).last()
      .evaluate((el) => (el.textContent || '').length)
      .catch(() => 0);
    if (len !== lastLen) {
      lastLen = len;
      lastChange = Date.now();
    }

    // Progress events: fire onProgress whenever a new markdown-content phase
    // appears (DR generates multiple answer blocks as it works).
    if (onProgress) {
      const phaseCount = await page.locator(SELECTORS.answerContainer).count().catch(() => 0);
      if (phaseCount > lastPhaseCount) {
        lastPhaseCount = phaseCount;
        onProgress(`phase ${phaseCount}`);
      }
    }

    if (len > 0 && Date.now() - lastChange >= stableMs) {
      return;
    }
    await page.waitForTimeout(800);
  }
  throw new PerplexityTimeoutError('answer-stable-total', totalTimeoutMs);
}

export async function scrapeOnce({ prompt, model = 'best', tool, focus = 'web', threadId, onProgress } = {}) {
  const { browser, page } = await launchAndNavigate({ focus, threadId });
  try {
    await selectModel(page, model);
    await selectTool(page, tool);

    const input = page.locator(SELECTORS.promptInput).first();
    await humanClick(page, input);
    await humanType(page, prompt);
    await humanPause(page, 200, 500);
    await page.keyboard.press(SELECTORS.submitKey);

    const isDeep = tool === 'deep-research';
    await waitForAnswerStable(page, {
      totalTimeoutMs: isDeep ? DEEP_TIMEOUT_MS : FAST_TIMEOUT_MS,
      stableMs: isDeep ? 15_000 : 3_000,
      onProgress,
    });

    // Optional extra settle for DR so late citations/steps land.
    if (isDeep) await humanPause(page, 2_000, 4_000);

    const html = await page.content();
    const url = page.url();
    return { html, url };
  } finally {
    await browser.close();
  }
}
