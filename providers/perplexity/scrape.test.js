import { describe, it, expect } from 'vitest';
import { launchAndNavigate, selectModel, selectTool, scrapeOnce } from './scrape.js';

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

describe.skipIf(!smoke)('scrape — model selection (SMOKE=1)', () => {
  it('selects Claude from the Model menu without error', async () => {
    const { page, browser } = await launchAndNavigate();
    try {
      await selectModel(page, 'claude');
      expect(true).toBe(true);  // would have thrown otherwise
    } finally {
      await browser.close();
    }
  }, 60_000);
});

describe.skipIf(!smoke)('scrape — tool selection (SMOKE=1)', () => {
  it('enables Deep research via the + menu without error', async () => {
    const { page, browser } = await launchAndNavigate();
    try {
      await selectTool(page, 'deep-research');
      expect(true).toBe(true);
    } finally {
      await browser.close();
    }
  }, 60_000);
});

describe.skipIf(!smoke)('scrape — full fast-path (SMOKE=1)', () => {
  it('submits a trivial prompt and returns an answer-bearing HTML payload', async () => {
    const { html, url } = await scrapeOnce({
      prompt: 'who founded meta (formerly facebook)?',
      model: 'best',
      focus: 'web',
    });
    expect(typeof html).toBe('string');
    expect(html.length).toBeGreaterThan(50_000);  // real app HTML is hundreds of KB
    expect(url).toMatch(/perplexity\.ai\/search\//);
    // The captured markdown-content div should have substantive answer text.
    expect(html).toMatch(/markdown-content-/);
  }, 240_000);
});
