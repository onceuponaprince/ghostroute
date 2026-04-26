# ghostroute

A context-hygiene layer for side-LLM integration. Ghostroute is a monorepo of provider scrapers that let other LLMs feed into, or emerge from, a Claude Code session without the user pasting entire transcripts and blowing Claude's context budget.

Every provider reuses an authenticated browser session instead of a paid API key. The trade is deliberate: API credits expire and API shapes change, but a cookie-driven session-reuse scraper keeps working as long as the user can log in. See [`docs/architecture.md`](docs/architecture.md) for the full rationale.

Two repos, N providers. Ghostroute is the monorepo. `fast-travel-cli` (Gemini, read-side) lives outside as a sibling Rust CLI at [`~/code/fast-travel-cli/`](https://github.com/onceuponaprince/fast-travel-cli).

## Components

### Node scrapers (this repo root)

- [`server.js`](server.js) — Express reverse-API. Exposes scrapers as HTTP endpoints Claude can call. See [`docs/server.md`](docs/server.md). Now also hosts `/ask-perplexity`, `/ask-perplexity/deep`, and `/ask-perplexity/deep/:jobId`.
- [`grok-reverse-api-grok-main.js`](grok-reverse-api-grok-main.js) — Current `askGrok()` implementation. Playwright + stealth plugin, cookie-reuse, human-paced typing.
- [`providers/perplexity/`](providers/perplexity/) — Structured Perplexity provider module (parse/scrape/jobs/errors split). Parse layer unit-tested against HTML fixtures; scrape layer smoke-tested live behind `SMOKE=1`.
- `grok-reverse-api.js`, `composable-scraper.js`, `search-scraper.js` — Earlier iterations and experiments. See [`docs/architecture.md#node-file-archaeology`](docs/architecture.md#node-file-archaeology).

### Rust CLIs

- [`ask-grok-cli/`](ask-grok-cli/) — Terminal-first Grok client on `chromiumoxide`. Usable standalone or orchestrated by Claude Code. See its [README](ask-grok-cli/README.md).
- [`ask-perplexity-cli/`](ask-perplexity-cli/) — Terminal-first Perplexity client on `chromiumoxide`. Mirrors the Node provider's scraping strategy and output shape; does not depend on a running server. Includes `--deep` (Deep Research, synchronous with progress to stderr), `--model`, `--focus`, `--thread`, `--raw`. See its [README](ask-perplexity-cli/README.md).

### Chrome extension

- [`cookie-master-key/`](cookie-master-key/) — Exports session cookies from any authenticated tab. URL-scoped so parent-domain cookies (consent, auth) are included — which is what makes Google-property scrapers work at all. See its [README](cookie-master-key/README.md).

### Design records

- [`docs/superpowers/specs/`](docs/superpowers/specs/) — Design specs (monorepo setup, Perplexity scraper, others).
- [`docs/superpowers/plans/`](docs/superpowers/plans/) — Implementation plans the specs drive.

## Providers

### Grok — shipped

Two surfaces:

- **HTTP endpoint.** Start `server.js`; POST to `/ask-grok` with `{prompt}`. Returns `{result}`.
- **Rust CLI.** `ask-grok-cli --prompt "..."` — same substrate, different entry point. Supports project-local memory at `<git-root>/.claude/.swarm-memory.json` and Claude Code orchestration hooks.

Both share the same cookie-reuse pattern and target `grok.com` via `grok.com-cookies.json`.

### Perplexity — shipped

Three surfaces:

- **HTTP sync.** `POST /ask-perplexity` with `{prompt, model?, tool?, focus?, threadId?, raw?}`. Returns `{answer, sources[], threadId, steps?, raw?}`.
- **HTTP async (Deep Research).** `POST /ask-perplexity/deep` returns `{jobId}` in <1s; poll `GET /ask-perplexity/deep/:jobId` until `status: "done"`.
- **Rust CLI.** `ask-perplexity-cli "prompt" [--model ...] [--focus ...] [--deep] [--thread ...] [--raw]` — same output JSON shape; `--deep` blocks synchronously with progress to stderr.

Design splits along Perplexity's response-time characteristics: fast modes (~30–90s) use the askGrok browser-roundtrip pattern with adaptive text-stabilisation completion detection; Deep Research (3–8 min) gets a job-shaped API on the HTTP side. Both run headed Chromium with stealth flags (Cloudflare + Perplexity Pro gate both detect headless).

Full taxonomy:
- `model`: `best` · `sonar` · `gpt` · `gemini` · `claude` · `kimi` · `nemotron`
- `tool`: `deep-research` (optional)
- `focus`: `web` · `academic` · `finance` · `health` · `patents` (URL-routed entry pages)

See [`docs/superpowers/specs/2026-04-23-perplexity-scraper-design.md`](docs/superpowers/specs/2026-04-23-perplexity-scraper-design.md) for the full spec including the Revisions log.

### Future providers

Drop in as `providers/<name>/`. The monorepo shape scales by provider, not by spawning new repos. See [`docs/architecture.md#monorepo-of-providers`](docs/architecture.md#monorepo-of-providers).

## Setup

1. **Node 20+.** `npm install` at the root.
2. **Install cookie-master-key** as an unpacked Chrome extension. See [`cookie-master-key/README.md`](cookie-master-key/README.md).
3. **Export cookies** for each provider you use. Move exports to `~/.claude/cookie-configs/<hostname>-cookies.json`.
4. **Start the server** (Grok via HTTP): `npm start`. Defaults to port 3005; auto-increments on `EADDRINUSE` up to 10 tries. Override with `PORT=XXXX npm start`.

## Usage

### Grok via HTTP

```bash
curl -X POST http://localhost:3005/ask-grok \
  -H "Content-Type: application/json" \
  -d '{"prompt":"what is the current Ethereum block height?"}'
```

Returns `{"result": "..."}`.

### Grok via Rust CLI

```bash
cd ask-grok-cli
cargo run --release -- --prompt "Write a short haiku about Rust."
```

See [`ask-grok-cli/README.md`](ask-grok-cli/README.md) for flags, memory conventions, and Claude Code integration.

### Perplexity via HTTP

```bash
# Fast request — any model, any focus
curl -X POST http://localhost:3005/ask-perplexity \
  -H 'Content-Type: application/json' \
  -d '{"prompt":"who founded meta?","model":"claude","focus":"academic"}' | jq

# Deep Research (async, ~3–8 min)
JOB=$(curl -sX POST http://localhost:3005/ask-perplexity/deep \
  -H 'Content-Type: application/json' \
  -d '{"prompt":"state of fusion energy"}' | jq -r .jobId)
curl -s http://localhost:3005/ask-perplexity/deep/$JOB | jq
```

### Perplexity via Rust CLI

```bash
cd ask-perplexity-cli
./target/release/ask-perplexity-cli --model claude --focus academic "who founded meta?"
./target/release/ask-perplexity-cli --deep "state of fusion energy"
```

## Architecture overview

Short form — the full version is in [`docs/architecture.md`](docs/architecture.md).

- **Node for scraper libraries.** Playwright's session management and `puppeteer-extra-stealth` integration are mature; the scraping surface has not earned a switch to Rust.
- **Rust for user-facing CLIs.** Compiled single-binary tools feel right for terminal use. `chromiumoxide` gives Rust parity for headless Chromium automation.
- **Session-reuse over API keys.** API credits die when they die; cookie-reuse survives as long as the user can log in.
- **Monorepo of providers.** Each side-LLM lands as `providers/<name>/`. New providers do not spawn new repos.

## Provider contract

A provider is whatever code reaches into a side-LLM's authenticated browser session and brings something back. Every provider in this repo speaks the same contract on three axes — input, output, transport — so the layer composes uniformly even as individual scrapers diverge in their internals.

### Input

A plain JavaScript object (Node) or a `clap`-parsed args struct (Rust). Required fields are minimal — usually a prompt or a target URL. Optional fields tune model selection, focus area, thread continuation, or raw-passthrough flags. Providers do not accept secrets in the call: cookies live at `~/.claude/cookie-configs/<hostname>-cookies.json` and are read at scrape time.

```js
// Node — providers/perplexity/index.js
askPerplexity({ prompt, model, tool, focus, threadId, raw })
```

```rust
// Rust — ask-perplexity-cli
ask-perplexity-cli "<prompt>" [--model ...] [--focus ...] [--thread ...] [--deep] [--raw]
```

### Output

A single JSON object. The minimum shape is `{ answer | result }` for prompt-and-return providers and `{ markdown }` for pure-read providers. Providers that surface citations add `sources[]`. Providers with multi-stage progress (Perplexity Deep Research) add `steps[]`. Providers that expose the raw upstream response add `raw`. Errors throw on the Node side and surface as non-zero exit codes plus stderr on the Rust side; HTTP transports translate to 4xx/5xx with `{ error, details? }`.

The contract is extensible by addition. New optional fields land alongside existing ones; established field names (`answer`, `sources`, `threadId`, `steps`, `raw`) carry the same meaning across providers.

### Transport

Three shapes, picked per-provider by what fits:

- **HTTP** — `server.js` mounts each provider as an Express endpoint. Synchronous requests return the JSON shape directly. Long-running requests (Deep Research) split into a `POST` that returns `{ jobId }` immediately and a `GET /:jobId` that polls.
- **Shell** — Each Rust CLI is a single binary that emits JSON to stdout, progress to stderr. Pipe directly into `jq`. Exit code zero on success, non-zero on failure with the cause on stderr.
- **Stdio (planned)** — MCP wrapping over the same provider functions. Tracked under the delegate-agent scenes in the Command Centre campaign; not yet wired.

A provider can support multiple transports without rewriting its core logic — the function lives in `providers/<name>/index.js` and the transports are thin wrappers over it.

## Add a provider

Drop a side-LLM into the monorepo by following the shape Perplexity established. The walkthrough below assumes you are adding `providers/<name>/` with a Node scraper; an accompanying Rust CLI sibling is optional and lands as `ask-<name>-cli/` at the repo root.

### Directory shape

```
providers/<name>/
├── index.js              # exports the public function (e.g. askFoo)
├── scrape.js             # browser automation: navigate, type, wait, extract HTML
├── parse.js              # pure HTML → JSON; no network, fully unit-testable
├── selectors.js          # CSS selectors as named constants (one place to fix on UI drift)
├── human.js              # typing jitter, click trails, pause helpers (copy from siblings)
├── jobs.js               # only if the provider has long-running async modes
├── errors.js             # named error classes the transport layer can switch on
├── __fixtures__/         # captured HTML samples for parse-layer unit tests
│   └── *.html
├── parse.test.js         # parse layer against fixtures (fast; runs by default)
├── scrape.test.js        # live scrape (skipped by default; SMOKE=1 to enable)
└── README.md             # provider-specific doc; see existing providers for shape
```

### Required surface

- **`index.js`** must export a single async function whose name follows `ask<Name>` and whose signature is `({ prompt, ...options })`. The function returns the JSON object documented in the provider contract.
- **`parse.js`** must be pure — no network, no browser. Given an HTML string and a URL, it returns the JSON. This is what the unit tests exercise.
- **`scrape.js`** owns the browser. It uses `puppeteer-extra` with the stealth plugin, reads cookies from `~/.claude/cookie-configs/<hostname>-cookies.json`, and runs **headed** by default. Most provider Pro features detect the `headless` flag.
- **`selectors.js`** keeps every CSS selector as an exported constant. When the provider's UI shifts, this is the only file that should need editing.

### Wiring transports

- **HTTP.** Add an endpoint to `server.js` that imports `ask<Name>` from `providers/<name>/index.js` and translates `req.body` into its options. Return the result as JSON. Errors caught by the route handler become 5xx responses with `{ error, details }`.
- **Rust CLI.** Add `ask-<name>-cli/` at the repo root with its own `Cargo.toml`. Mirror `ask-perplexity-cli/`'s structure — `chromiumoxide` for browser, `clap` for args, `serde_json` for output. The Rust CLI does not depend on the Node provider; both reach the same upstream surface.

### Tests

- **Parse tests run by default.** `npm test` exercises every `*.test.js` file. Parse-layer tests load fixtures from `__fixtures__/` and assert on the JSON shape — no browser, no network, sub-second runs.
- **Scrape tests are smoke-gated.** `SMOKE=1 npm run test:e2e:perplexity` runs the live scrape against the real provider; the same pattern applies for new providers (`test:e2e:<name>`). These need cookies and a display.
- **Capture fixtures from real responses.** When you build a new provider, save a few real HTML responses under `__fixtures__/` — enough to cover the response variants the parser must handle. Strip personal data before committing.

### Documentation

- Add `providers/<name>/README.md` describing the provider's state (skeleton / scaffolded / shipped), the input options it accepts, the JSON shape it emits, and any quirks (e.g. Perplexity hides the model menu on `/academic` routes).
- Reference the new provider from the root README's Components and Providers sections.
- If the provider is non-trivial, add a design spec under `docs/superpowers/specs/<date>-<provider>-scraper-design.md` before writing the code.

## Conventions

- **Cookies** live outside the repo at `~/.claude/cookie-configs/<hostname>-cookies.json`. Never committed.
- **Design records** live under `docs/superpowers/specs/` and `docs/superpowers/plans/`.
- **Feature branches** use worktrees at `.worktrees/feature/<name>/`. Ignored via `.gitignore`.
- **Atomic commits.** One logical change per commit; see the monorepo setup design for the initial four-commit split.
- **Headed Chromium.** The Perplexity scraper (Node + Rust) runs headed by default — Cloudflare and Perplexity's Pro-feature gate both detect `headless`. On headless servers, run under `Xvfb`.

## License

MIT.
