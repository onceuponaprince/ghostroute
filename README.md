# ghostroute

Tools for scraping and automating LLM web UIs. Currently X.com Grok and
perplexity.ai.

## Components

- **`./` — Node.js scraper** · Express-based reverse-API using Playwright.
  Entry: `node server.js`. Exposes `/ask-grok`, `/ask-perplexity`, and the
  async pair `/ask-perplexity/deep` + `/ask-perplexity/deep/:jobId` for
  Deep Research jobs. See `providers/perplexity/` for the structured
  provider module (parse layer unit-tested against HTML fixtures; scrape
  layer smoke-tested live behind `SMOKE=1`).
- **`ask-grok-cli/` — Rust CLI** · Terminal-first Grok client built on
  `chromiumoxide`. Usable standalone or orchestrated by Claude Code.
- **`ask-perplexity-cli/` — Rust CLI** · Terminal-first Perplexity client
  built on `chromiumoxide`. Mirrors the Node provider's scraping strategy
  and output shape; does not depend on a running server. Includes
  `--deep` (Deep Research, synchronous with progress to stderr), `--model`,
  `--focus`, `--thread`, `--raw`.
- **`cookie-master-key/` — Chrome extension** · Exports session cookies
  from x.com / grok.com / perplexity.ai in the format the scrapers expect.

## Perplexity usage

```bash
# Fast request — any model, any focus (Web/Academic/Finance/Health/Patents)
curl -X POST http://localhost:3005/ask-perplexity \
  -H 'Content-Type: application/json' \
  -d '{"prompt":"your question","model":"claude","focus":"academic"}' | jq

# Deep Research (async, can take several minutes)
JOB=$(curl -sX POST http://localhost:3005/ask-perplexity/deep \
  -H 'Content-Type: application/json' \
  -d '{"prompt":"your research question"}' | jq -r .jobId)

# Poll status until status == "done"
curl -s http://localhost:3005/ask-perplexity/deep/$JOB | jq
```

Request fields:

| Field       | Values                                                              | Default |
| ---         | ---                                                                 | ---     |
| `prompt`    | any string                                                          | —       |
| `model`     | `best` · `sonar` · `gpt` · `gemini` · `claude` · `kimi` · `nemotron` | `best`  |
| `tool`      | `deep-research` (optional; if set, use the async `/deep` endpoint)  | none    |
| `focus`     | `web` · `academic` · `finance` · `health` · `patents`               | `web`   |
| `threadId`  | continue an existing thread by UUID                                 | —       |
| `raw`       | include raw HTML in the response                                    | `false` |

Response shape: `{ answer, sources[], threadId, steps?, raw? }`. See
`docs/superpowers/specs/2026-04-23-perplexity-scraper-design.md` for
details including the Revisions log.

## Shared conventions

- Cookies live outside the repo in `~/.claude/cookie-configs/` (never
  committed).
- The Perplexity scraper runs chromium **headed** by default — Cloudflare
  and Perplexity's Pro-feature gate both detect headless Chromium. On a
  headless server, run the scraper under `Xvfb`.

See each sub-project's README for setup specifics.
