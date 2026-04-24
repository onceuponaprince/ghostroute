// Perplexity bot-gate probe.
//
// Launches a browser, loads saved cookies, navigates to perplexity.ai,
// and reports whether the page loaded cleanly or hit a verify-human gate.
//
// Usage:
//   node scripts/perplexity-probe.mjs                    # stock playwright, headed
//   node scripts/perplexity-probe.mjs --headless         # stock playwright, headless
//   node scripts/perplexity-probe.mjs --patchright       # stealth-patched drop-in
//   node scripts/perplexity-probe.mjs --patchright --headless
//
// Expected cookies at: ~/.claude/cookie-configs/perplexity.ai-cookies.json
//
// Outputs:
//   - JSON telemetry to stdout
//   - Screenshot to ./probe-<timestamp>.png
//   - Keeps the browser open for 30s so you can inspect interactively

import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

const args = new Set(process.argv.slice(2));
const headless = args.has('--headless');
const usePatchright = args.has('--patchright');

const cookiePath = path.join(os.homedir(), '.claude', 'cookie-configs', 'perplexity.ai-cookies.json');
if (!fs.existsSync(cookiePath)) {
  console.error(`Cookies not found at ${cookiePath}`);
  process.exit(1);
}

const driver = usePatchright ? 'patchright' : 'playwright';
console.error(`[probe] driver=${driver} headless=${headless}`);

const { chromium } = await import(driver);

const browser = await chromium.launch({ headless });
const context = await browser.newContext({
  viewport: { width: 1440, height: 900 },
  userAgent: 'Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36',
});

const cookies = JSON.parse(fs.readFileSync(cookiePath, 'utf8'));
await context.addCookies(cookies);

const page = await context.newPage();
const t0 = Date.now();

try {
  await page.goto('https://www.perplexity.ai/', { waitUntil: 'domcontentloaded', timeout: 30_000 });
} catch (err) {
  console.error('[probe] navigation error:', err.message);
}

// Give Cloudflare/Turnstile time to appear if it's going to.
await page.waitForTimeout(5_000);

const telemetry = await page.evaluate(() => {
  const markers = {
    // App loaded correctly → prompt input exists
    hasPromptInput: !!document.querySelector('div[contenteditable="true"][role="textbox"]'),
    // Signed-in? A signed-in Perplexity shouldn't show this.
    hasSignInButton: !!Array.from(document.querySelectorAll('button, a')).find(
      (el) => /sign\s*in|log\s*in/i.test(el.textContent || '')
    ),
    // Cloudflare challenge markers
    hasTurnstileIframe: !!document.querySelector('iframe[src*="challenges.cloudflare.com"], iframe[src*="turnstile"]'),
    hasCfChallengeText: /verify\s+you\s+are\s+human|just a moment|enable javascript and cookies/i.test(document.body.innerText || ''),
    // Generic bot-gate text
    hasVerifyHumanText: /verify\s+you\s+are\s+human/i.test(document.body.innerText || ''),
    // Any iframe at all (often a tell for gated pages)
    iframeCount: document.querySelectorAll('iframe').length,
    iframeSrcs: Array.from(document.querySelectorAll('iframe')).map((i) => i.src).slice(0, 5),
    // Who does the browser think it is
    navigatorWebdriver: navigator.webdriver,
    userAgent: navigator.userAgent.slice(0, 120),
    // Page basics
    title: document.title,
    bodyTextPreview: (document.body.innerText || '').slice(0, 200),
  };
  return markers;
});

const finalUrl = page.url();
const screenshotPath = path.resolve(process.cwd(), `probe-${driver}${headless ? '-headless' : ''}.png`);
await page.screenshot({ path: screenshotPath, fullPage: false });

const report = {
  driver,
  headless,
  finalUrl,
  elapsedMs: Date.now() - t0,
  screenshot: screenshotPath,
  verdict: telemetry.hasPromptInput
    ? 'APP_LOADED'
    : telemetry.hasTurnstileIframe || telemetry.hasCfChallengeText
      ? 'BLOCKED_BY_CHALLENGE'
      : telemetry.hasSignInButton
        ? 'LOGGED_OUT'
        : 'UNKNOWN_STATE',
  telemetry,
};

console.log(JSON.stringify(report, null, 2));

// Keep open so user can observe / solve a challenge / confirm state.
console.error('[probe] browser will stay open for 30s — inspect interactively');
await page.waitForTimeout(30_000);

await browser.close();
