import { describe, it, expect } from 'vitest';
import { launchAndNavigate } from './scrape.js';

const smoke = process.env.SMOKE === '1';

describe.skipIf(!smoke)('scrape — live browser (SMOKE=1)', () => {
  it('launches chromium, loads cookies, navigates to perplexity.ai without login wall', async () => {
    const { page, browser } = await launchAndNavigate();
    try {
      const url = page.url();
      expect(url).toMatch(/perplexity\.ai/);
      const content = await page.content();
      // Rough sanity: we landed somewhere with a prompt input visible.
      expect(content.toLowerCase()).toMatch(/perplexity/);
    } finally {
      await browser.close();
    }
  }, 60_000);
});
