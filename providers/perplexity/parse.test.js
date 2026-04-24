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
