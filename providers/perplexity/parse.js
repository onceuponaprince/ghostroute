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

  return {
    answer,
    sources: [],
    threadId: null,
  };
}
