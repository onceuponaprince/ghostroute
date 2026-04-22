# ghostroute

Tools for scraping and automating LLM web UIs, starting with X.com Grok.

## Components

- **`./` — Node.js scraper** · Express-based reverse-API approach using
  Playwright and puppeteer-extra-stealth. Entry: `node server.js`.
- **`ask-grok-cli/` — Rust CLI** · Terminal-first Grok client built on
  `chromiumoxide`. Usable standalone or orchestrated by Claude Code.
- **`cookie-master-key/` — Chrome extension** · Exports session cookies
  from x.com / grok.com in the format the scrapers expect.

## Shared conventions

- Cookies live outside the repo in `~/.claude/cookie-configs/` (never committed).

See each sub-project's README for setup specifics.
