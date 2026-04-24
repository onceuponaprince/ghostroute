import { describe, it, expect } from 'vitest';
import { launchAndNavigate } from './scrape.js';

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
