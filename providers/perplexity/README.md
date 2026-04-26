# providers/perplexity

**State:** shipped. Two transports — HTTP via [`../../server.js`](../../server.js), and a Rust CLI sibling at [`../../ask-perplexity-cli/`](../../ask-perplexity-cli/) that targets the same upstream surface independently. Parse layer is unit-tested against captured HTML fixtures; the scrape layer has a smoke test gated by `SMOKE=1`.

This is the reference shape for any new provider. The directory is intentionally flat — every file owns a single responsibility, and the parse/scrape split is what makes the parser fast to test without a browser.

## File layout

| File | Lines | Role |
|------|-------|------|
| [`index.js`](index.js) | 30 | Public entry. Exports `askPerplexity()` (sync) and `askPerplexityDeep()` (job-shaped). |
| [`scrape.js`](scrape.js) | 217 | Browser automation — launch, navigate, select model, select tool, type, wait, return HTML. The only file that touches Playwright. |
| [`parse.js`](parse.js) | 78 | Pure HTML → JSON. Cheerio-based. No network, no browser, fully unit-testable. |
| [`selectors.js`](selectors.js) | 66 | Every CSS selector as a named constant. The fragile layer — UI drift edits land here and only here. |
| [`human.js`](human.js) | 79 | Typing jitter, click trails, pause helpers. Defence-in-depth against automation detection. |
| [`jobs.js`](jobs.js) | 47 | In-memory job store for Deep Research. `create()` / `updateProgress()` / `complete()` / `fail()` / `get()`, with TTL-scheduled GC. |
| [`errors.js`](errors.js) | 35 | Named error classes — `PerplexityAuthError`, `PerplexityScrapeError`, `PerplexityTimeoutError`, `PerplexityParseError`. The transport layer switches on these. |
| [`__fixtures__/`](__fixtures__/) | — | Captured HTML samples (`auto-web.html`, `reasoning-web.html`, `deep-research-web.html`). Drive the parse-layer tests. |
| `*.test.js` | — | Co-located vitest suites. Parse and unit tests run by default; `scrape.test.js` is `SMOKE=1`-gated. |

## Contract

### Input

```js
askPerplexity({
  prompt,                     // required string
  model = 'best',             // 'best' | 'sonar' | 'gpt' | 'gemini' | 'claude' | 'kimi' | 'nemotron'
  tool,                       // 'deep-research' | undefined
  focus = 'web',              // 'web' | 'academic' | 'finance' | 'health' | 'patents'
  threadId,                   // continue an existing /search/<slug> thread
  raw = false,                // include raw HTML blobs in the output
})

askPerplexityDeep({
  prompt, model, focus, threadId, raw,
  store,                      // job store from createJobStore()
})
```

`askPerplexity()` returns a Promise resolving to the JSON object below. `askPerplexityDeep()` returns `{ jobId }` synchronously and runs the scrape on `setImmediate`; the job store carries progress and final result.

### Output

```json
{
  "answer": "Final response prose.",
  "sources": [
    { "index": 1, "title": "...", "url": "https://...", "domain": "example.com", "snippet": "..." }
  ],
  "threadId": "abc-123-uuid",
  "steps": [
    { "query": "Identifying ...", "phase": "identifying" }
  ],
  "raw": {
    "answerHtml": "<div>...</div>",
    "sourcesHtml": "<div>...</div>"
  }
}
```

| Field | Always? | Notes |
|-------|---------|-------|
| `answer` | yes | Final prose. |
| `sources` | yes | `[]` when nothing was browsed. |
| `threadId` | yes | `null` when the resolved URL is not a `/search/<slug>` page. |
| `steps` | Deep Research only | Each step has `query` and `phase` (`identifying` / `searching` / `insights` / `other`). Phase is mapped from the icon ID via [`selectors.js`](selectors.js)' `PHASE_BY_ICON` table. |
| `raw` | only when `raw: true` | Raw HTML for the answer and sources containers, for downstream parsing or debugging. |

### Transports

- **HTTP sync.** `POST /ask-perplexity` with the input fields. Returns the JSON shape directly. Wrapped in [`../../server.js`](../../server.js).
- **HTTP async.** `POST /ask-perplexity/deep` returns `{ jobId }` in under a second; poll `GET /ask-perplexity/deep/:jobId` until `status: "done"`. The job store TTL-cleans completed jobs after 24 hours.
- **Shell.** `ask-perplexity-cli "..."` — the Rust CLI sibling; same JSON shape, same scraping strategy, no dependency on a running server. See [`../../ask-perplexity-cli/README.md`](../../ask-perplexity-cli/README.md).

## Cookies

Export from an authenticated `perplexity.ai` tab using `cookie-master-key` and move the file to `~/.claude/cookie-configs/perplexity.ai-cookies.json`. If the file is missing, `scrape.js` throws `PerplexityAuthError` at launch with the path it expected.

A Perplexity Pro subscription is required for several models and for Deep Research. Free-tier cookies will land on a stub interface where the model menu shows only Sonar.

## Detection bypass

Perplexity gates Pro features behind automation-detection checks. The scrape layer does five things to keep the gate open:

1. Runs **headed** — Cloudflare detects the `headless` flag directly. On headless servers, run under `Xvfb`.
2. Suppresses `--enable-automation` in the Chromium launch args.
3. Passes `--disable-blink-features=AutomationControlled`.
4. Injects an init script overriding `navigator.webdriver`, `window.chrome`, and `navigator.languages`.
5. Uses [`human.js`](human.js)'s typing jitter, occasional typos with corrections, and mouse-move trails before clicks.

Without these, Perplexity serves the stub interface mentioned above. With them, all seven models surface and Deep Research is selectable.

## Tests

```bash
# Default — parse + unit tests, no browser, sub-second
npm test

# Live smoke — scrapes Perplexity for real, requires cookies + display
SMOKE=1 npm run test:e2e:perplexity
```

The parse layer is exercised against [`__fixtures__/`](__fixtures__/). Add new fixtures when Perplexity's HTML shape shifts; strip personal data before committing.

## Known limitations

- **Deep Research source panel not captured.** The Node provider does not currently extract sources from Deep Research output; the Rust CLI inherits the limitation. The fixture is in place; the parser branch is the next beat.
- **Thread follow-ups can return the original answer.** `parse()` uses `.last()` to pick the most recent answer block; in some thread-state shapes the wrong block is selected. Reproducible enough to log, not yet root-caused.
- **`focus=academic|finance|health|patents` ignores `--model`.** Those routes hide the model menu entirely; Perplexity uses its default for the topic. The CLI accepts the flag silently rather than erroring, which is friendly but invisible. A future revision should warn.
- **`tool` is `'deep-research'` only.** Other tool slots (image, code interpreter) are not wired. Adding them would extend `selectTool()` in [`scrape.js`](scrape.js) and the `tool` taxonomy in the README.

## Related

- [`../../README.md`](../../README.md) — repo-level README; Components and Providers sections list both Perplexity surfaces.
- [`../../docs/superpowers/specs/2026-04-23-perplexity-scraper-design.md`](../../docs/superpowers/specs/2026-04-23-perplexity-scraper-design.md) — the design spec, including the Revisions log that captures what changed between the initial sketch and the shipped layout.
- [`../../docs/superpowers/plans/2026-04-23-perplexity-node-provider.md`](../../docs/superpowers/plans/2026-04-23-perplexity-node-provider.md) — the implementation plan this directory was built against.
- [`../../ask-perplexity-cli/README.md`](../../ask-perplexity-cli/README.md) — Rust CLI sibling; same contract, different transport.
