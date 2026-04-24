import { describe, it, expect } from 'vitest';
import { readFileSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { parse } from './parse.js';

const __dirname = dirname(fileURLToPath(import.meta.url));
const fixture = (name) =>
  readFileSync(resolve(__dirname, '__fixtures__', name), 'utf8');

describe('parse — answer extraction', () => {
  it('extracts a non-empty answer from auto-web fixture', () => {
    const result = parse(fixture('auto-web.html'), { url: 'https://perplexity.ai/search/abc123' });
    expect(typeof result.answer).toBe('string');
    expect(result.answer.length).toBeGreaterThan(20);
  });

  it('answer references Meta/Facebook founder for the seed prompt', () => {
    const result = parse(fixture('auto-web.html'), { url: 'https://perplexity.ai/search/abc123' });
    const answer = result.answer.toLowerCase();
    expect(answer).toMatch(/zuckerberg|meta|facebook/);
  });
});

describe('parse — sources extraction', () => {
  it('extracts at least one source from auto-web fixture', () => {
    const result = parse(fixture('auto-web.html'), { url: 'https://perplexity.ai/search/abc' });
    expect(Array.isArray(result.sources)).toBe(true);
    expect(result.sources.length).toBeGreaterThanOrEqual(1);
  });

  it('each source has index, title, url, domain', () => {
    const result = parse(fixture('auto-web.html'), { url: 'https://perplexity.ai/search/abc' });
    for (const source of result.sources) {
      expect(typeof source.index).toBe('number');
      expect(typeof source.title).toBe('string');
      expect(source.url).toMatch(/^https?:\/\//);
      expect(source.domain).toMatch(/\./);
    }
  });

  it('source indices are 1-based and sequential', () => {
    const result = parse(fixture('auto-web.html'), { url: 'https://perplexity.ai/search/abc' });
    result.sources.forEach((s, i) => {
      expect(s.index).toBe(i + 1);
    });
  });
});
