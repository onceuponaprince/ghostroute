import { scrapeOnce } from './scrape.js';
import { parse } from './parse.js';

export async function askPerplexity({ prompt, model = 'best', tool, focus = 'web', threadId, raw = false }) {
  const { html, url } = await scrapeOnce({ prompt, model, tool, focus, threadId });
  return parse(html, { url, mode: tool === 'deep-research' ? 'deep-research' : undefined, raw });
}

export function askPerplexityDeep({ prompt, model = 'best', focus = 'web', threadId, raw = false, store }) {
  const { jobId } = store.create();

  setImmediate(async () => {
    try {
      const { html, url } = await scrapeOnce({
        prompt,
        model,
        tool: 'deep-research',
        focus,
        threadId,
        onProgress: (text) => store.updateProgress(jobId, text),
      });
      const result = parse(html, { url, mode: 'deep-research', raw });
      store.complete(jobId, result);
    } catch (err) {
      store.fail(jobId, err);
    }
  });

  return { jobId };
}
