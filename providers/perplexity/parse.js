import * as cheerio from 'cheerio';
import { PerplexityParseError } from './errors.js';
import { SELECTORS, PHASE_BY_ICON } from './selectors.js';

export function parse(html, { url, mode } = {}) {
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
  const result = { answer, sources, threadId: null };

  if (mode === 'deep-research') {
    result.steps = extractSteps($);
  }

  return result;
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

function extractSteps($) {
  const items = $(SELECTORS.stepItem);
  const out = [];
  items.each((_, el) => {
    const $el = $(el);
    const query = $el.find(SELECTORS.stepQuery).first().text().trim();
    const iconEl = $el.find(SELECTORS.stepPhaseIcon).first();
    const iconRef = iconEl.attr('xlink:href') || iconEl.attr('href');
    const phase = PHASE_BY_ICON[iconRef] || 'other';
    if (query) out.push({ query, phase });
  });
  return out;
}
