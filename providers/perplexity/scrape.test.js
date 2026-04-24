import { describe, it, expect } from 'vitest';
import { launchAndNavigate, selectMode } from './scrape.js';

const smoke = process.env.SMOKE === '1';

describe.skipIf(!smoke)('scrape — live browser (SMOKE=1)', () => {
  it('launches chromium, loads cookies, navigates to perplexity.ai without login wall', async () => {
    const { page, browser } = await launchAndNavigate();
    try {
      // Confirm we got past Cloudflare (not stuck on __cf_chl_rt_tk)
      expect(page.url()).not.toContain('__cf_chl_rt_tk');
      expect(page.url()).toMatch(/perplexity\.ai/);
      // Confirm the real app loaded — the Lexical prompt input is distinctive
      // and not present on the Cloudflare challenge page.
      const hasPromptInput = await page.locator('div[contenteditable="true"][role="textbox"]').count();
      expect(hasPromptInput).toBeGreaterThan(0);
    } finally {
      await browser.close();
    }
  }, 60_000);

  it('focus: academic routes to /academic entry URL', async () => {
    const { page, browser } = await launchAndNavigate({ focus: 'academic' });
    try {
      expect(page.url()).toMatch(/perplexity\.ai\/academic/);
    } finally {
      await browser.close();
    }
  }, 60_000);
});

// Mode selection tests are SKIPPED pending spec re-scope. The current
// Perplexity UI has no "Pro" / "Reasoning" / "Deep Research" buttons — it
// has a Model menu with 7 models (Best, Sonar, GPT-5.4, Gemini 3.1 Pro,
// Claude Sonnet 4.6, Kimi K2.6, Nemotron 3 Super) and no visible Deep
// Research toggle. See PROGRESS.md / scratch notes.
describe.skip('scrape — mode selection (BLOCKED on spec re-scope)', () => {
  it('selects Pro mode via the + menu without error', async () => {
    const { page, browser } = await launchAndNavigate();
    try {
      await selectMode(page, 'pro');
      expect(true).toBe(true);
    } finally {
      await browser.close();
    }
  }, 60_000);

  it('selects Deep Research via dedicated button', async () => {
    const { page, browser } = await launchAndNavigate();
    try {
      await selectMode(page, 'deep-research');
      expect(true).toBe(true);
    } finally {
      await browser.close();
    }
  }, 60_000);
});
