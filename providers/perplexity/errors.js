export class PerplexityAuthError extends Error {
  constructor() {
    super('Perplexity login wall hit — refresh cookies at ~/.claude/cookie-configs/perplexity.ai-cookies.json');
    this.name = 'PerplexityAuthError';
  }
}

export class PerplexityScrapeError extends Error {
  constructor(stage, selector, html) {
    super(`Perplexity scrape failed at stage "${stage}" (selector: ${selector})`);
    this.name = 'PerplexityScrapeError';
    this.stage = stage;
    this.selector = selector;
    this.html = html;
    this.htmlTruncated = (html || '').slice(0, 4096);
  }
}

export class PerplexityTimeoutError extends Error {
  constructor(stage, timeoutMs) {
    super(`Perplexity timed out at stage "${stage}" after ${timeoutMs}ms`);
    this.name = 'PerplexityTimeoutError';
    this.stage = stage;
    this.timeoutMs = timeoutMs;
  }
}

export class PerplexityParseError extends Error {
  constructor(reason, html) {
    super(`Perplexity parse failed: ${reason}`);
    this.name = 'PerplexityParseError';
    this.reason = reason;
    this.html = html;
  }
}
