# Architecture

The architectural rationale behind ghostroute. This is the deep-dive companion to the root [`README.md`](../README.md)'s short-form architecture overview.

---

## The context-hygiene layer

Modern solo development runs multiple LLM sessions concurrently — one for building, another for cross-check, a third for web-grounded research. Each model has strengths the others do not. The problem is that each model's conversation is a walled garden: carrying a Gemini research thread into Claude, or a Grok answer into a Claude-Code session, means pasting transcripts. Transcripts are large. Claude's context budget is finite. A single pasted Gemini Deep Research output will eat ten thousand tokens of working memory that could have been task state.

Ghostroute installs a layer between the other LLMs and Claude. Instead of pasting, the user invokes a scraper. The scraper returns the specific fragment Claude needs — a range, a final answer, a cleaned markdown dump — and the rest of the transcript stays out of Claude's window. This is the hygiene trade: context is spent on the work, not on carrying state between tools.

Two directional shapes live in the layer:

- **Prompt-and-return.** Claude writes a prompt, the scraper invokes a side-LLM browser session, the answer comes back. `ask-grok-cli` and the Node server's `/ask-grok` endpoint do this.
- **Pure read.** The user has already had a conversation in a side-LLM browser tab; the scraper reads it and emits markdown. `fast-travel-cli` does this for Gemini.

Both shapes share the same substrate — authenticated browser sessions, cookie-reuse, headless Chromium — so they land as siblings inside the same layer.

---

## Node vs Rust: the language split

The split is honest, not doctrinal. Pick the language that fits the tool's centre of gravity; do not force uniformity.

**Node for scraper libraries.** Ghostroute's Node side originated from `grok-reverse-api` work that was already JS-shaped when the repo was founded. Playwright's session management, `puppeteer-extra-stealth` plugin for bot-detection evasion, and cheerio for HTML parsing are all first-class in Node. The scraping surface itself — HTTP server, DOM traversal, request shaping — has not earned a switch to Rust. The Node layer is where the provider logic lives.

**Rust for user-facing CLIs.** Compiled single-binary tools feel right for terminal utilities invoked directly. `chromiumoxide` gives Rust parity for headless Chromium automation via the Chrome DevTools Protocol. `clap` gives it a terse, derive-friendly CLI parser. `tokio` gives it async. The Rust CLI side (`ask-grok-cli`, `ask-perplexity-cli`, `fast-travel-cli`) is where user-facing tools live, all pinned to the same `chromiumoxide` baseline.

The outcome is a two-language split that accepts the upfront cost of maintaining both toolchains in exchange for each half fitting its job. The two sides communicate through the same cookie-jar format and the same filesystem conventions, so adding a new provider on one side does not require changing the other.

---

## Session-reuse over API keys

The design principle all providers share: **never depend on a paid API tier.**

The lesson came from earlier work documented in Scene 1.05 of the Command Centre campaign — the `delegate-agent` routing chain that reached Grok via API keys stopped working when the backing xAI credits were exhausted. API-key paths die when credits die. API shapes also change: an API redesign can break consumers overnight, with no recourse other than rewriting.

Session-reuse paths survive longer. As long as the user can log into the browser UI, the scraper works. Cookies are harvested once (via `cookie-master-key`), placed at `~/.claude/cookie-configs/<hostname>-cookies.json`, and reused across runs until they expire. Cookie expiry is months, not days. When cookies do expire, a re-export is a one-click fix.

The trade-off is fragility against UI change. If the side-LLM's DOM shifts, the extractor breaks, and the fix is selector maintenance rather than API migration. In practice DOM selectors live in a single `page.evaluate` string per provider, so a break is a single-edit fix. API migrations tend to cascade.

This principle also shaped `cookie-master-key`. The extension was deliberately scoped to export URL-applicable cookies (including parent-domain cookies like Google's `.google.com` session cookies), not hostname-narrow exports. Without that, the entire Google-property scraper family — Gemini, Drive, Docs, YouTube — is blocked on consent walls and sign-in redirects. One cookie-export fix unblocks an entire provider family.

---

## Monorepo of providers

The 2026-04-22 monorepo setup design ([`docs/superpowers/specs/2026-04-22-ghostroute-monorepo-setup-design.md`](superpowers/specs/2026-04-22-ghostroute-monorepo-setup-design.md)) established ghostroute's shape: a single flat repo that houses every side-LLM scraper as a peer.

The shape was confirmed on 2026-04-23 when Perplexity work landed inside ghostroute rather than spawning `~/code/perplexity-scraper/`. Perplexity's provider directory (`providers/perplexity/`) sits alongside `ask-grok-cli/` and the root Node scrapers. Future side-LLMs (Claude.ai, ChatGPT, YouTube transcripts, Drive documents) drop in as `providers/<name>/` with no new repo creation.

The shape is deliberately flat rather than workspace-based (`apps/`, `packages/`). Two reasons:

1. **Start flat, upgrade later.** Workspace tooling (npm workspaces, Nx, Turborepo) is valuable when real cross-package duplication or shared dependencies emerge. Ghostroute has not hit that point. Premature workspace adoption is cost without benefit.
2. **Non-destructive upgrade path.** Moving from flat to workspace is a straightforward refactor when the second duplication appears. Starting with workspaces and discovering they are overkill costs more.

Each provider is self-contained: its own parse logic, its own fixtures, its own Rust CLI sibling if one exists. The only shared surface is the cookie-jar convention at `~/.claude/cookie-configs/<hostname>-cookies.json` and the design-spec folder at `docs/superpowers/`.

---

## Node file archaeology

The repo root has more `.js` files than the current design requires. This is historical layering, not active complexity. The files fall into three groups:

**Current, load-bearing:**

- [`server.js`](../server.js) — Express HTTP server. Imports `askGrok` from `grok-reverse-api-grok-main.js` and exposes it at `POST /ask-grok`. See [`server.md`](server.md).
- [`grok-reverse-api-grok-main.js`](../grok-reverse-api-grok-main.js) — Current `askGrok()` implementation. Playwright + stealth + cookie-reuse + human-paced typing.

**Earlier iterations, not imported:**

- [`grok-reverse-api.js`](../grok-reverse-api.js) — An earlier askGrok draft. Same substrate, different structure; superseded by `grok-reverse-api-grok-main.js` when the human-paced-typing logic matured.
- [`composable-scraper.js`](../composable-scraper.js) — A base-class experiment — a `BaseScraper` class meant to host pluggable extraction logic. The monorepo-of-providers pattern made the inheritance approach unnecessary; providers self-contain instead of sharing a base.

**Standalone experiment:**

- [`search-scraper.js`](../search-scraper.js) — A DuckDuckGo HTML-search scraper using cheerio. Not wired into the server; stands alone as a reference for non-browser scraping. Kept because the cheerio-over-HTML pattern may land in a future provider where JavaScript rendering is not needed.

These files are retained rather than deleted because each encodes a real design attempt that informed where the current code settled. A future cleanup pass may relocate them to `docs/archaeology/` or prune them once enough providers exist to re-test whether any of their patterns want a second life.

---

## Related

- [`README.md`](../README.md) — root entry with setup, usage, and component list.
- [`server.md`](server.md) — `server.js` deep dive.
- [`superpowers/specs/2026-04-22-ghostroute-monorepo-setup-design.md`](superpowers/specs/2026-04-22-ghostroute-monorepo-setup-design.md) — initial monorepo design.
- [`superpowers/specs/2026-04-23-perplexity-scraper-design.md`](superpowers/specs/2026-04-23-perplexity-scraper-design.md) — second-provider design that confirmed the monorepo shape.
- [`superpowers/specs/2026-04-27-fast-travel-cli-design.md`](superpowers/specs/2026-04-27-fast-travel-cli-design.md) — Gemini read-side surface; first migrated-in component.
