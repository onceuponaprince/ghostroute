# ghostroute — perplexity scraper design

**Date:** 2026-04-23
**Status:** approved, pre-implementation
**Scope:** add Perplexity as the second LLM scraping target, with full-fidelity
output (answer + cited sources + raw HTML escape hatch), Deep Research support
via an async job pattern, optional threading, and a Rust CLI sibling.

---

## Context

`ghostroute` currently scrapes one target (Grok) via two surfaces: a Node
Express reverse-API (`server.js` → `askGrok`) and a Rust CLI (`ask-grok-cli/`,
`chromiumoxide`). The Grok flow is a single monolithic function: open browser,
inject cookies, type prompt, grab `innerText`, close browser, return string.

Perplexity differs from Grok in three ways that change the design:

1. **Citations are the product.** Discarding sources reduces Perplexity to a
   worse ChatGPT wrapper. Return shape must be structured.
2. **Modes are not equal.** Auto / Pro / Reasoning complete in ≤60s. Deep
   Research runs 3–8 minutes and streams intermediate progress. These cannot
   share one HTTP endpoint shape — the synchronous Grok pattern breaks behind
   any proxy with a default 60s gateway timeout.
3. **Threads are URL-addressable.** A Perplexity thread can be continued by
   navigating back to its URL. Grok has no analogue.

## Decisions

| #  | Decision | Rationale |
|----|----------|-----------|
| 1  | **Full-fidelity scraping** (answer + sources + mode + focus + Deep Research + threading), not a minimal `askGrok` mirror. | User selected option C in scoping. Sources are Perplexity's actual differentiator. |
| 2  | **Hybrid sync+async** — fast modes synchronous, Deep Research async with job queue. | Deep Research's 3–8 min runtime breaks any synchronous HTTP contract. Job queue isolates the long-running path. |
| 3  | **Structured output with raw HTML escape hatch** (B+D hybrid). | Parsed happy path for callers; raw HTML as debug telemetry and future-proof fallback when selectors drift. |
| 4  | **In-memory job map** for Deep Research, no persistence. | YAGNI for single-user scraper. Documented limitation — swap to SQLite when durability bites. |
| 5  | **Optional threading via client-held `threadId`**, server stateless. | Works identically for sync and async modes. No server-side session state; horizontally scalable. |
| 6  | **Provider module pattern** (`providers/perplexity/`) now, provider registry later. | Isolates selectors in one file (80% of the maintenance value of a registry) without touching working Grok code. Grok stays at root until a third provider forces the refactor. |
| 7  | **Cookies at `~/.claude/cookie-configs/perplexity.ai-cookies.json`**, shared by Node + Rust. | Follows global CLAUDE.md convention. Grok's in-repo cookie file is legacy. |
| 8  | **Rust CLI sibling `ask-perplexity-cli/`**, native `chromiumoxide` (not shelling into Node). | Consistency with `ask-grok-cli/`. Shelling into Node would require Node at runtime and defeat the point of a Rust sibling. |
| 9  | **Focus expanded to Web / Academic / Finance / Health / Patents, implemented via URL navigation (e.g. `/academic`, `/finance`). Writing dropped.** | 2026-04-24 revision (see §Revisions): Perplexity's UI no longer exposes a per-query focus button. Topic filters are now top-level URL-routable paths. Writing (LLM-only mode) has no current URL equivalent and is removed. Net effect is broader coverage via simpler (URL-based) implementation. |
| 10 | **Modes limited to Auto / Pro / Reasoning** (fast) + **Deep Research** (async). | These are the Pro-tier modes that materially differ from one another. |

## Architecture

### Final repo layout (additions only)

```
ghostroute/
├── server.js                    (modified — imports providers/perplexity, adds endpoints)
├── providers/
│   └── perplexity/
│       ├── index.js             public API: askPerplexity, askPerplexityDeep
│       ├── scrape.js            browser lifecycle, navigation, DOM interaction
│       ├── selectors.js         ALL Perplexity DOM selectors (the fragile layer)
│       ├── parse.js             HTML → { answer, sources, steps }
│       ├── jobs.js              in-memory Map<jobId, JobState> + TTL GC
│       ├── errors.js            typed error classes
│       ├── __fixtures__/        saved HTML from real responses for parse tests
│       └── perplexity.test.js   vitest — parse.js unit tests on fixtures
├── ask-perplexity-cli/          (new Rust sibling)
│   ├── Cargo.toml
│   ├── README.md
│   ├── .gitignore
│   └── src/
│       ├── main.rs              clap CLI
│       ├── scraper.rs           chromiumoxide browser + navigation
│       ├── selectors.rs         mirrors Node selectors.js
│       └── parse.rs             HTML → structured output (via scraper crate)
```

### Data flow — fast modes (Auto / Pro / Reasoning)

```
client
  └── POST /ask-perplexity { prompt, mode?, focus?, threadId?, raw? }
        └── server.js handler
              └── providers/perplexity/index.js :: askPerplexity()
                    └── scrape.js
                          ├── launch chromium
                          ├── load cookies from ~/.claude/cookie-configs/
                          ├── navigate to perplexity.ai OR perplexity.ai/search/<threadId>
                          ├── click mode selector
                          ├── click focus filter
                          ├── type prompt, press Enter
                          ├── wait for completion selector
                          ├── page.content() → html
                          └── close browser
                    └── parse.js :: parse(html) → { answer, sources, threadId, raw? }
              └── res.json(result)
```

### Data flow — Deep Research (async)

```
client
  ├── POST /ask-perplexity/deep { prompt, focus?, threadId? }
  │     └── server.js handler
  │           ├── jobs.create() → jobId, status: 'queued'
  │           ├── queueMicrotask(() => runDeepResearch(jobId, ...))
  │           └── res.json({ jobId })
  │
  ├── GET /ask-perplexity/deep/:jobId
  │     └── res.json(jobs.get(jobId))     // { status, progress?, result?, error? }
  │
  └── [background] runDeepResearch(jobId, opts)
        └── providers/perplexity/index.js :: askPerplexityDeep()
              ├── (same browser lifecycle as fast modes, with DR mode selected)
              ├── while (not done) { scrape progress text → jobs.updateProgress(jobId, text) }
              ├── on completion: parse → jobs.complete(jobId, result)
              └── on error: jobs.fail(jobId, err)
```

## API surface

### HTTP (extends `server.js`)

```
POST /ask-perplexity
  body:  { prompt: string,
           mode?: 'auto' | 'pro' | 'reasoning',           // default: 'auto'
           focus?: 'web' | 'academic' | 'finance' | 'health' | 'patents',  // default: 'web' (→ entry URL)
           threadId?: string,                             // continue existing thread
           raw?: boolean }                                // include raw HTML — default: false
  → 200: { answer, sources[], threadId, raw? }
  → 401: { error: 'PerplexityAuthError', message: 'refresh cookies' }
  → 502: { error: 'PerplexityScrapeError', stage, selector }
  → 504: { error: 'PerplexityTimeoutError', stage }

POST /ask-perplexity/deep
  body:  { prompt: string,
           focus?: 'web' | 'academic' | 'finance' | 'health' | 'patents',  // → entry URL
           threadId?: string }
  → 202: { jobId: string }

GET /ask-perplexity/deep/:jobId
  → 200: { status: 'queued' | 'running' | 'done' | 'failed',
           progress?: string,                             // e.g. "Searching 12 sources"
           result?: { answer, sources[], steps[], threadId, raw? },
           error?: string }
  → 404: { error: 'JobNotFound' }                         // unknown or GC'd jobId
```

### Rust CLI (`ask-perplexity-cli`)

```
ask-perplexity "prompt"                          # fast, Auto mode, Web focus
ask-perplexity --mode pro --focus academic "..."
ask-perplexity --deep "..."                      # Deep Research, blocks with progress prints to stderr
ask-perplexity --thread <uuid> "follow-up..."
ask-perplexity --raw "..."                       # include raw HTML in JSON output
```

Output: the same JSON shape as the HTTP response, pretty-printed to stdout. Progress updates (during `--deep`) print to stderr prefixed with `[progress]` so stdout stays pipe-safe for `| jq`.

**`--deep` jobId behavior:** on start, the CLI generates a UUID v4 and prints `[job] <uuid>` to stderr, and includes it as a top-level `jobId` field in the final JSON. This is a correlation identifier for humans tailing logs — it is *not* a recovery handle (the Rust CLI runs its own browser; it does not use the HTTP server's job queue). The HTTP `/deep` endpoint's `jobId` is a true recovery handle; these two ids look the same but mean different things.

## Return shape

```js
{
  answer: string,                    // pure prose (Perplexity UI no longer renders inline citation markers — see §Revisions)
  sources: [
    { index: 1, title: string, url: string, domain: string, snippet?: string }
  ],
  steps?: [                          // Deep Research only
    { query: string, phase: 'identifying' | 'searching' | 'insights' | 'other' }
  ],
  threadId: string,                  // from URL after submission
  raw?: {                            // only when raw: true in request
    answerHtml: string,
    sourcesHtml: string
  }
}
```

Focus is implemented by navigating to the corresponding entry URL before
submitting the prompt: `web` → `/`, `academic` → `/academic`, `finance` →
`/finance`, `health` → `/health`, `patents` → `/patents`. Mode selection
(Auto/Pro/Reasoning/Deep Research) is orthogonal and happens after entry
navigation. Continuing an existing thread (via `threadId`) navigates to
`/search/<threadId>` directly, ignoring focus.

## Threading

- Client holds `threadId`. Server stores nothing.
- First call omits `threadId`: scraper navigates to `https://perplexity.ai`,
  submits prompt, extracts UUID from the resulting URL
  (`https://perplexity.ai/search/<uuid>`), returns it.
- Subsequent calls pass `threadId`: scraper navigates to
  `https://perplexity.ai/search/<threadId>`, submits follow-up in the same
  thread's input box, returns the same `threadId`.
- Threading works identically for fast modes and Deep Research.

## Error handling

Typed errors in `providers/perplexity/errors.js`:

| Error | When | HTTP status | Behavior |
|-------|------|-------------|----------|
| `PerplexityAuthError` | Login wall detected before prompt submission | 401 | Message: "refresh cookies at ~/.claude/cookie-configs/perplexity.ai-cookies.json" |
| `PerplexityScrapeError` | Required selector not found | 502 | Attach `stage`, `selector`, and `html` (truncated to 4KB) for debugging |
| `PerplexityTimeoutError` | Fast mode >3min end-to-end, or Deep Research >30min end-to-end (no progress update for 5min also trips it) | 504 | Attach `stage` (which wait timed out) |
| `PerplexityParseError` | HTML present but parse failed | 502 *or* returns partial result with `parseError` field if `raw: true` | When caller opted in to `raw`, prefer returning raw HTML + parseError over throwing |

**Never swallow errors silently.** Always log with enough context to reproduce
(prompt, mode, focus, threadId) — scrubbing any PII if present in the prompt
becomes the caller's responsibility; we log what we were given.

**Deep Research job failure:** background task sets job status to `failed` with
error message. Poll returns `{ status: 'failed', error }`. The error is never
lost.

## Testing

- **`parse.js` unit tests** run against saved HTML fixtures in
  `__fixtures__/`. Fixtures captured during implementation from real Perplexity
  responses (one per mode × focus combination we support). No browser needed in
  CI. This is the part that must work — structured output correctness.
- **Smoke integration test** (`pnpm test:e2e:perplexity`) runs the full pipeline
  against a trivial prompt (`"what is 2+2"`). Skipped in CI (no cookies). Runs
  locally to catch selector drift.
- **No unit tests for `scrape.js`.** Thin wrapper around Playwright — mocking
  the browser is more code than the module itself. Smoke test covers it.
- **Rust CLI:** a single `cargo test` covering `parse.rs` against the same
  fixture HTML files the Node tests use.

## Known fragilities (to verify during implementation)

1. **Perplexity selectors are unknown at design time.** Discover via
   `playwright codegen` against a real Pro session. Put every selector in
   `selectors.js`. Prefer `data-testid` and `aria-label` over class names.
2. **Deep Research completion signal** needs DOM investigation. Likely
   candidates: a specific "finished" status element, absence of the generating
   spinner, or a stable message count. Documented in `selectors.js` with a
   comment explaining the chosen signal and what to change if it breaks.
3. **Stealth.** Perplexity is far less aggressive about fingerprinting than
   X/Grok. Start without `puppeteer-extra-stealth` on Playwright; add it only
   if we see blocks. Rust `chromiumoxide` has no stealth plugin — if
   fingerprinting bites there, add manually. Node first, Rust ports after Node
   proves the approach.
4. **Mode/focus UI changes across viewports.** Some of these controls live
   behind a dropdown at narrow widths. Lock viewport to 1440×900 in both
   Node and Rust scrapers to eliminate that variable.

## Implementation order

Designed so the repo is never in a broken intermediate state:

1. **Cookies + fixture capture.** Export Perplexity cookies via
   `cookie-master-key`, place at
   `~/.claude/cookie-configs/perplexity.ai-cookies.json`. Manually scrape one
   response per mode × focus combination, save HTML as fixtures in
   `__fixtures__/`.
2. **`parse.js` + unit tests against fixtures.** Zero browser involvement. Get
   structured output right first.
3. **`scrape.js` + `selectors.js`** — browser lifecycle, fast modes only.
   Manual smoke test.
4. **Wire into `server.js`** — `/ask-perplexity` live, fast modes end-to-end.
5. **Threading** — add `threadId` round-trip. Verify follow-up context lands.
6. **Deep Research** — `jobs.js`, `/ask-perplexity/deep*` endpoints,
   background task, progress polling.
7. **Rust CLI** — port selectors, ship fast modes first, Deep Research second.

## Out of scope

- **Generalized provider registry** (option B from scoping). Deferred until a
  third LLM scraper forces the abstraction.
- **Job persistence** (SQLite / Redis for Deep Research jobs). In-memory Map
  with documented restart-loses-job limitation.
- **Rate limiting on `/ask-perplexity*`.** Single-user scraper; Perplexity's own
  Pro-tier rate limits are the ceiling that matters.
- **Refactoring Grok code** into the new provider pattern. Grok stays as-is.
- **CI integration tests.** E2E test runs locally only; no cookies in CI.
## Verification checklist

After implementation, the following should be true:

- [ ] `POST /ask-perplexity { prompt: "what is 2+2" }` returns a valid
      structured payload within 90s.
- [ ] `sources[]` contains ≥1 entry with `{ index, title, url, domain }`.
- [ ] Passing `threadId` from a prior response continues that thread (verify
      the model references prior turns).
- [ ] `POST /ask-perplexity/deep` returns a `jobId` in <1s.
- [ ] `GET /ask-perplexity/deep/:jobId` transitions `queued → running → done`
      over a single Deep Research run.
- [ ] Completed job returns `result.steps[]` with ≥1 entry.
- [ ] `mode: 'reasoning'` and `focus: 'academic'` each produce distinguishable
      outputs vs. defaults (reasoning shows chain-of-thought; academic links to
      arXiv/PubMed-class domains via `/academic` entry URL).
- [ ] `focus: 'finance'` resolves by navigating to `/finance` and produces
      finance-biased sources for a relevant query.
- [ ] `raw: true` in request yields populated `raw.answerHtml` and
      `raw.sourcesHtml`.
- [ ] Missing/expired cookies yield HTTP 401 with the "refresh cookies"
      message, not a generic 500.
- [ ] `ask-perplexity "prompt"` from the Rust CLI produces matching structured
      JSON output for a trivial prompt.
- [ ] `parse.js` vitest suite passes against every fixture in `__fixtures__/`.
- [ ] `providers/perplexity/__fixtures__/` is committed; cookie files are not.

---

## Revisions

### 2026-04-24 — inline citation markers dropped

Offline dissection of captured fixtures (auto-web, reasoning-web,
deep-research-web) via `scripts/dissect-fixtures.mjs` found **zero**
inline citation markers in the answer prose: no `[N]` text patterns,
no `<a>` tags, no `<sup>` tags. Perplexity's current UI renders the
answer as pure prose, with sources listed separately in the overlay
panel (no numbered annotations in situ).

**Change:** `answer` return-shape field is pure prose. Plan Task 6
("parse.js — inline citation markers preserved") deleted. Verification
checklist item removed.

Sources still extracted fully and indexed 1..N in `sources[]` as before.
Callers that need citation-style rendering can cross-reference sources
by position or by content-matching titles against the answer text.

### 2026-04-24 — `steps[].pagesVisited` replaced with `steps[].phase`

DR step dissection found no visible pages-visited count in the UI.
Each step is an icon + text pair; pages visited is not exposed. Instead
the icon identifies the step's phase:

- `#pplx-icon-blocks` → `identifying`
- `#pplx-icon-world-search` → `searching`
- `#pplx-icon-bolt` → `insights`
- anything else → `other`

Return shape field renamed: `steps[].pagesVisited` → `steps[].phase`.
`stepPagesCount` selector replaced by `stepPhaseIcon` (read the
`xlink:href` of the `<svg use>` inside the step button).

### 2026-04-24 — focus filter re-scoped as URL navigation

Live sidebar probe (`scripts/sidebar-probe.mjs`) found Perplexity no
longer exposes a per-query focus button. Topic filtering is implemented
as top-level URL-routable paths: `/academic`, `/finance`, `/health`,
`/patents`, `/discover`, and the root `/`. Each is a pre-filtered search
entry page.

**Changes:**

1. `focus` enum becomes `web | academic | finance | health | patents`.
   `writing` is dropped — Perplexity no longer exposes an LLM-only
   mode via any URL. `finance`, `health`, and `patents` are new (were
   previously deemed gimmicks — the URL-based implementation makes them
   cheap to support).
2. Focus selection moves from `selectFocus()` DOM click to entry-URL
   navigation. No focus button selector is needed. `FOCUS_URLS = { web:
   '/', academic: '/academic', finance: '/finance', health: '/health',
   patents: '/patents' }`.
3. Task 12 `selectFocus()` can be deleted; focus is applied via
   `page.goto('https://www.perplexity.ai' + FOCUS_URLS[focus])` in
   `launchAndNavigate`.
4. `writing-focus.html` fixture removed from Task 2 requirements (no
   longer captures meaningful state since Writing mode is gone).
5. "Writing mode in Deep Research" out-of-scope item removed (the
   combination is now impossible by construction).

Simpler implementation, broader coverage.
