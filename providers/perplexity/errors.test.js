import { describe, it, expect } from 'vitest';
import {
  PerplexityAuthError,
  PerplexityScrapeError,
  PerplexityTimeoutError,
  PerplexityParseError,
} from './errors.js';

describe('perplexity errors', () => {
  it('PerplexityAuthError carries the refresh-cookies message', () => {
    const err = new PerplexityAuthError();
    expect(err).toBeInstanceOf(Error);
    expect(err.name).toBe('PerplexityAuthError');
    expect(err.message).toMatch(/refresh cookies/i);
  });

  it('PerplexityScrapeError captures stage, selector, html', () => {
    const err = new PerplexityScrapeError('await-completion', 'div.done', '<html>...</html>');
    expect(err.name).toBe('PerplexityScrapeError');
    expect(err.stage).toBe('await-completion');
    expect(err.selector).toBe('div.done');
    expect(err.html).toBe('<html>...</html>');
  });

  it('PerplexityTimeoutError captures the stage that timed out', () => {
    const err = new PerplexityTimeoutError('await-completion', 180_000);
    expect(err.name).toBe('PerplexityTimeoutError');
    expect(err.stage).toBe('await-completion');
    expect(err.timeoutMs).toBe(180_000);
  });

  it('PerplexityParseError carries the original html for debugging', () => {
    const err = new PerplexityParseError('missing answer node', '<div></div>');
    expect(err.name).toBe('PerplexityParseError');
    expect(err.html).toBe('<div></div>');
  });

  it('Scrape error truncates html to 4KB for server logs', () => {
    const big = 'x'.repeat(10_000);
    const err = new PerplexityScrapeError('stage', 'sel', big);
    expect(err.htmlTruncated.length).toBeLessThanOrEqual(4096);
  });
});
