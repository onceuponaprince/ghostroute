# ghostroute — fast-travel-cli design

**Date:** 2026-04-27 (retroactive — code shipped 2026-04-22 to 2026-04-26 in a sibling repo, migrated into ghostroute on 2026-04-27)
**Status:** approved, post-implementation, migrated into monorepo
**Scope:** Gemini *read* surface — extract an existing conversation by URL and emit markdown to stdout. The inverse direction of `ask-grok-cli` and `ask-perplexity-cli`, which write a prompt and return an answer.

---

## Context

ghostroute's existing CLIs (`ask-grok-cli`, `ask-perplexity-cli`) and Node provider (`providers/perplexity/`) share one pattern: write a prompt into the provider's web UI, wait for a response, return it. fast-travel-cli inverts the direction. The user has already had a conversation in another tab; the tool's job is to get that conversation *into* a Claude Code session without the user pasting a giant transcript.

That changes what "good" looks like compared to the ask-* CLIs:

1. **No prompt typing.** No human-paced typing, no Drunk-Typist protocol, no response-stabilisation polling. Just navigate and read.
2. **Conversation by URL is the input.** Gemini's `https://gemini.google.com/app/<conversation-id>` is the addressable handle for any past conversation. No threadId state to manage.
3. **Output is the user's full transcript.** Markdown with `## User` / `## Model` headers — pipe-friendly into `claude --resume` or a file.

## Decisions

| #  | Decision | Rationale |
|----|----------|-----------|
| 1  | **Session-reuse cookies, not API key.** Cookies live at `~/.claude/cookie-configs/gemini.google.com-cookies.json`. | Same reason every other ghostroute provider uses cookies: API credits expire when they expire; cookie-reuse survives as long as the user can log in. Gemini's API also has stricter rate limits than the web UI for free-tier users. |
| 2  | **Headless Chromium by default.** Not headed. | Unlike Perplexity (Cloudflare gate) and Grok (challenge gate), Gemini doesn't actively detect `headless`. The headed-by-default cost — visible window, slower startup, flicker — would buy nothing here. `--visible` flag exists as a debug affordance. |
| 3  | **Single-file `src/main.rs`.** No modular split into `browser/`, `automation/`, `cli/`, `config/`. | First-run scope is read-only — no typing automation, no response stabilisation, no thread management, no memory file. The `ask-grok-cli` shape exists because Grok's flow needed locator helpers, human typing, and response-detail collection. fast-travel-cli has none of those. Will split when `main.rs` crosses ~400 lines. |
| 4  | **`page.evaluate` for extraction, not selector-by-selector traversal.** | One JS block runs in-page, returns `{messages: [{role, content}]}`. Rust deserialises. Selector drift updates one place. |
| 5  | **Output shape: bare `## User` / `## Model` markdown.** No frontmatter, no citation list, no mode annotations. | The downstream consumer is Claude — Claude doesn't need metadata to read a transcript. Frontmatter would force the user to strip it; citation extraction is Perplexity's domain, not Gemini's. |
| 6  | **Auth-redirect detection before DOM polling.** | Stale or analytics-only cookies cause Gemini to redirect to `accounts.google.com/signin`. Polling for the conversation DOM in that case stalls for the full 30s timeout. Detecting the redirect immediately gives a legible error pointing at cookie re-export. |
| 7  | **`window.scrollTo(0, 0)` once before extraction.** | Gemini lazily renders older messages. Without scrolling to top once, virtualised messages may be silently truncated. Single scroll handles most cases; heavier virtualisation would need a scroll-loop. |
| 8  | **CDP cookie injection requires a real domain navigation first.** Navigate to `https://gemini.google.com` *before* `set_cookies`. | CDP rejects `set_cookies` calls on `about:blank`. `chromiumoxide` opens new tabs at `about:blank` by default. |
| 9  | **No tests on first-run.** | The surface is one happy path against a third-party DOM. Tests become valuable when extraction logic gains branches (range selection, format flags). Today, a passing test would only prove that selectors haven't drifted *yet*. |

## Architecture

### File layout

```
fast-travel-cli/
├── Cargo.toml             chromiumoxide 0.9.1, clap derive, tokio full, serde
├── Cargo.lock
├── README.md
├── .gitignore             /target, .worktrees/, .claude/
└── src/
    └── main.rs            ~320 lines: load_cookies, launch_browser,
                           inject_cookies_and_navigate, wait_for_conversation,
                           dump_diagnostics, extract_conversation, render_markdown
```

### Data flow

```
user invocation
  └── fast-travel-cli --conversation-url <url>
        └── load_cookies                          reads ~/.claude/cookie-configs/gemini.google.com-cookies.json
        └── launch_browser                        chromiumoxide headless (or --visible)
        └── inject_cookies_and_navigate
              ├── new_page("https://gemini.google.com")    domain context for CDP
              ├── set_cookies                              apply session
              ├── reload                                   pick up auth state
              ├── auth-redirect early-exit                 fail fast on signin redirect
              ├── goto(conversation_url)                   navigate to actual target
              └── wait_for_navigation
        └── wait_for_conversation                 polls user-query / model-response, ~30s timeout
                                                   on timeout: dump_diagnostics (URL, title, selector counts, body preview)
        └── extract_conversation                  page.evaluate JS returns {messages}
        └── render_markdown                       stdout, role-prefixed headers
```

## Trade-offs

- **Headless ≠ undetectable.** Gemini doesn't fight headless today. If Google adds detection later, the fix is to default `--visible` on — not retrofit a stealth-plugin layer like Perplexity needs.
- **No incremental output.** The tool runs to completion before printing, which means a long conversation waits until extraction finishes. For the read-side use case this is acceptable; for write-side it would not be.
- **Single-conversation only.** No batch mode, no folder export. Adding either is a multiplier on top of the current core; deferring keeps first-run shape clean.
- **DOM selectors are tied to `<user-query>` / `<model-response>` custom elements.** When Gemini's UI updates these elements, the `extract_conversation` JS block needs updating in one place. The `dump_diagnostics()` fallback (when DOM polling times out) makes selector drift detectable without a manual probe.

## Revisions

### 2026-04-22 — Initial scaffold (`b6a8b60`, `8e81173`)
- Cargo project skeleton with `chromiumoxide = "0.9.1"`, `clap` with derive, `tokio` full, `serde` + `serde_json`.
- Edition 2024, matching `ask-grok-cli` and `ask-perplexity-cli`.
- `.gitignore` excludes `/target`, `.worktrees/`, `.claude/` from the start so session state and feature-branch worktrees never leak into commits.

### 2026-04-23 — First-run flow shipped (`5cb562a`)
- All five core functions landed in `src/main.rs`: `load_cookies`, `launch_browser`, `inject_cookies_and_navigate`, `wait_for_conversation`, `extract_conversation`, `render_markdown`.
- Auth-redirect early exit added inline in `inject_cookies_and_navigate` so the failure mode (analytics-only cookie file) surfaces a legible error pointing at cookie-master-key re-export.
- README documented Known Stalls (auth redirect, selector drift, virtualised rendering).

### 2026-04-25 — `--visible` + diagnostics (`1d000e7`)
- Added `--visible` flag for debug runs (`chromiumoxide::BrowserConfig::builder().with_head()`).
- Added `dump_diagnostics()` that runs on selector-timeout: reports URL, title, custom-element counts (top 15), body preview (first 800 chars, first 30 lines).
- Reasoning: when `wait_for_conversation` times out *without* diagnostics, the only signal is "30 seconds passed." With the dump, the user sees whether they hit a sign-in page, an empty body, or a real DOM-selector drift — distinct failure modes that need distinct fixes.
- 8s hold-open in `--visible` mode lets the user inspect the rendered DOM manually before the browser closes.

### 2026-04-26 — Selector narrowing (`86fdfe2`)
- Earlier extractor matched both outer custom elements (`user-query`, `model-response`) AND inner content classes (`.user-query-content`, `.model-response-text`).
- Result: every model turn was captured twice in the output.
- Fix: match only the outer custom elements. Added `stripLeadingLabels` helper to remove Gemini's UI labels ("You said", "Show thinking", "Gemini said") that appear inside `innerText`.

### 2026-04-27 — Migration into ghostroute monorepo (this revision)
- Subtree-merged into `ghostroute/fast-travel-cli/`. Original commits preserved in ghostroute's log via `git subtree add --prefix=fast-travel-cli ../fast-travel-cli main`. Resulting merge commit is `88482ca`.
- Cross-links in `fast-travel-cli/README.md` rewritten from external `github.com/onceuponaprince/ghostroute/...` paths to relative `../ask-grok-cli/`, `../cookie-master-key/`.
- chromiumoxide harmonised to `0.9.1` across all three Rust CLIs. `ask-grok-cli` and `ask-perplexity-cli` were pinned at 0.7. The 0.7 → 0.9 jump dropped the `tokio-runtime` feature flag — tokio is now an unconditional dependency in chromiumoxide. The two older crates' `features = ["tokio-runtime"]` clauses were removed; no source-code changes were required, both crates compiled clean at 0.9.1.
- Old repo (`github.com/onceuponaprince/fast-travel-cli`) to be archived with a redirect README pointing at this monorepo. Local copy at `~/code/fast-travel-cli/` to be deleted after the user verifies ghostroute build works end-to-end.

## Future work (out of scope for first-run)

- **Range selection.** `--from-message N --to-message M` for partial extraction. Useful when only the tail of a long Gemini conversation is relevant.
- **Output format flags.** `--format json` for programmatic consumption, `--format plain` for non-Claude downstreams.
- **Typed error taxonomy.** Currently a `Box<dyn Error + Send + Sync>` style. Replace with `thiserror` enum once failure modes settle. The diagnostics dump on timeout is what the typed errors will route to.
- **Gemini *write* surface** (`ask-gemini-cli`). Symmetric to `ask-grok-cli` and `ask-perplexity-cli`. Out of scope for this batch — see [the migration plan](../plans/2026-04-27-fast-travel-cli-migration.md) §Out of scope.
- **Test suite.** First-run shipped without tests because the surface is one happy path against a third-party DOM. Tests become valuable when extraction logic gains branches; until then, the diagnostics dump is the unit of debuggability.
- **Installed-binary packaging.** No `cargo install` story yet. First-run is exploratory; once the surface stabilises, package alongside `ask-grok-cli` and `ask-perplexity-cli`.
