import { describe, it, expect, vi } from 'vitest';
import { readFileSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const fixture = (name) => readFileSync(resolve(__dirname, '__fixtures__', name), 'utf8');

vi.mock('./scrape.js', () => ({
  scrapeOnce: vi.fn(),
}));

const { scrapeOnce } = await import('./scrape.js');
const { askPerplexity } = await import('./index.js');
import { createJobStore } from './jobs.js';

describe('askPerplexity', () => {
  it('returns parsed structured result', async () => {
    scrapeOnce.mockResolvedValueOnce({
      html: fixture('auto-web.html'),
      url: 'https://www.perplexity.ai/search/fixture-thread-id',
    });
    const result = await askPerplexity({ prompt: 'anything' });
    expect(result.answer.length).toBeGreaterThan(20);
    expect(result.sources.length).toBeGreaterThanOrEqual(1);
    expect(result.threadId).toBe('fixture-thread-id');
    expect(result.raw).toBeUndefined();
  });

  it('passes raw option through to parse', async () => {
    scrapeOnce.mockResolvedValueOnce({
      html: fixture('auto-web.html'),
      url: 'https://www.perplexity.ai/search/fixture-thread-id',
    });
    const result = await askPerplexity({ prompt: 'x', raw: true });
    expect(result.raw).toBeDefined();
    expect(typeof result.raw.answerHtml).toBe('string');
  });

  it('forwards model, tool, focus, threadId to scrapeOnce', async () => {
    scrapeOnce.mockResolvedValueOnce({
      html: fixture('auto-web.html'),
      url: 'https://www.perplexity.ai/search/continued',
    });
    await askPerplexity({
      prompt: 'x',
      model: 'claude',
      tool: 'deep-research',
      focus: 'academic',
      threadId: 'continued',
    });
    expect(scrapeOnce).toHaveBeenCalledWith(expect.objectContaining({
      prompt: 'x',
      model: 'claude',
      tool: 'deep-research',
      focus: 'academic',
      threadId: 'continued',
    }));
  });

  it('maps tool=deep-research to parse mode=deep-research for step extraction', async () => {
    scrapeOnce.mockResolvedValueOnce({
      html: fixture('deep-research-web.html'),
      url: 'https://www.perplexity.ai/search/dr-thread',
    });
    const result = await askPerplexity({ prompt: 'x', tool: 'deep-research' });
    expect(Array.isArray(result.steps)).toBe(true);
    expect(result.steps.length).toBeGreaterThanOrEqual(1);
  });
});

describe('askPerplexityDeep', () => {
  it('immediately returns jobId; job transitions queued → running → done', async () => {
    const { askPerplexityDeep } = await import('./index.js');
    const store = createJobStore();
    scrapeOnce.mockImplementationOnce(async ({ onProgress }) => {
      onProgress?.('Searching 3 sources');
      return {
        html: fixture('deep-research-web.html'),
        url: 'https://www.perplexity.ai/search/deep-thread',
      };
    });

    const { jobId } = askPerplexityDeep({ prompt: 'x', store });
    expect(store.get(jobId).status).toBe('queued');

    // Wait for completion (poll up to ~500ms)
    for (let i = 0; i < 50 && store.get(jobId).status !== 'done'; i++) {
      await new Promise((r) => setTimeout(r, 10));
    }
    const final = store.get(jobId);
    expect(final.status).toBe('done');
    expect(final.result.threadId).toBe('deep-thread');
  });

  it('on scrape failure the job status becomes failed', async () => {
    const { askPerplexityDeep } = await import('./index.js');
    const store = createJobStore();
    scrapeOnce.mockRejectedValueOnce(new Error('scrape boom'));

    const { jobId } = askPerplexityDeep({ prompt: 'x', store });
    for (let i = 0; i < 50 && store.get(jobId).status !== 'failed'; i++) {
      await new Promise((r) => setTimeout(r, 10));
    }
    expect(store.get(jobId)).toMatchObject({ status: 'failed', error: 'scrape boom' });
  });
});
