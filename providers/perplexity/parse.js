import * as cheerio from 'cheerio';
import { PerplexityParseError } from './errors.js';
import { SELECTORS } from './selectors.js';

export function parse(html, { url } = {}) {
  const $ = cheerio.load(html);

  const answerNode = $(SELECTORS.answerContainer).last();
  if (answerNode.length === 0) {
    throw new PerplexityParseError('answer container not found', html);
  }
  const answer = answerNode.text().trim();
  if (!answer) {
    throw new PerplexityParseError('answer container empty', html);
  }

  const sources = extractSources($);

  return {
    answer,
    sources,
    threadId: null,
  };
}

function extractSources($) {
  const items = $(SELECTORS.sourceItem);
  if (items.length === 0) return [];

  const out = [];
  items.each((i, el) => {
    const $el = $(el);
    const href = $el.attr('href');
    if (!href) return;
    let domain = '';
    try {
      domain = new URL(href).hostname;
    } catch {
      return;
    }
    const title = $el.find(SELECTORS.sourceTitle).first().text().trim() || domain;
    const snippet = $el.find(SELECTORS.sourceSnippet).first().text().trim() || undefined;
    out.push({ index: i + 1, title, url: href, domain, snippet });
  });
  return out;
}
