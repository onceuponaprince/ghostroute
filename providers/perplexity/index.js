import { scrapeOnce } from './scrape.js';
import { parse } from './parse.js';

export async function askPerplexity({ prompt, model = 'best', tool, focus = 'web', threadId, raw = false }) {
  const { html, url } = await scrapeOnce({ prompt, model, tool, focus, threadId });
  return parse(html, { url, mode: tool === 'deep-research' ? 'deep-research' : undefined, raw });
}
