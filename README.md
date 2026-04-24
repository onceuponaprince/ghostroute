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

## Conventions

- **Cookies** live outside the repo at `~/.claude/cookie-configs/<hostname>-cookies.json`. Never committed.
- **Design records** live under `docs/superpowers/specs/` and `docs/superpowers/plans/`.
- **Feature branches** use worktrees at `.worktrees/feature/<name>/`. Ignored via `.gitignore`.
- **Atomic commits.** One logical change per commit; see the monorepo setup design for the initial four-commit split.
- **Headed Chromium.** The Perplexity scraper (Node + Rust) runs headed by default — Cloudflare and Perplexity's Pro-feature gate both detect `headless`. On headless servers, run under `Xvfb`.

## License

MIT.
