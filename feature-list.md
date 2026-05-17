# ghostroute — feature list

The ghostroute workspace is a Rust Cargo workspace at `~/code/ghostroute/`
containing four crates:

- **`ghostroute-browser`** — shared chromiumoxide bootstrap, stealth,
  cookies, human-behaviour helpers, persistent profiles, interstitial
  detection, DOM-shape diagnostics. Single-version dependency for the CLIs.
- **`fast-travel-cli`** — read existing AI-chat conversations
  (Gemini, Claude, ChatGPT, Perplexity, Grok) into stdout markdown.
- **`ask-perplexity-cli`** — submit prompts to perplexity.ai and return
  cited answers as JSON. Supports thread continuation, deep research.
- **`ask-grok-cli`** — submit prompts to grok.com and return answers as text.

Cross-cutting source of truth for higher-level Spore design lives in
`~/code/build-in-public/docs/superpowers/feature-list.md` (BorAI Spore)
and the prior-art research at
`~/code/build-in-public/docs/superpowers/ghostroute-prior-art-2026-04-27.md`.

---

## What is shipped today

### Workspace + shared crate (2026-04-27)
- Cargo workspace at `~/code/ghostroute/Cargo.toml` consolidating four
  crates with shared dep versions (`chromiumoxide`, `tokio`, etc.).
- `ghostroute-browser` exports: `launch(LaunchOpts)`,
  `install_stealth(page)`, `STEALTH_INIT_SCRIPT`, `cookie_file_path`,
  `load_cookie_data`, `parse_cookies`, `human_pause(min, max)`,
  `sanitize_unicode_for_typing(text)`, `detect_interstitial(page)`,
  `capture_dump_dom(page)`, `default_profile_dir`, `ensure_profile_dir`,
  `ProfileDir`. Gemini extraction selectors stay per-CLI; everything
  shared lives here.

### `fast-travel-cli` features
- **5-provider auto-detection** via URL substring matching
  (`gemini.google.com`, `chatgpt.com|chat.openai.com`, `claude.ai`,
  `perplexity.ai`, `grok.com|x.com/i/grok`).
- **`--chrome-profile <name>`** — resolves a Chrome/Chromium profile by
  display name across `~/.config/google-chrome`,
  `~/.config/chromium`, `~/snap/chromium/common/chromium`, and
  `~/.config/google-chrome-for-testing`. Falls back to literal subdir
  match. Errors with a union listing across all installations.
- **`--profile-dir <path>`** — explicit ghostroute-managed profile dir.
- **`--init-all`** — opens visible Chromium with one tab per provider,
  blocks on stdin Enter / sentinel file `/tmp/ghostroute-init-done` /
  `--init-wait-secs N` (default 600). Optional `--init-providers
  claude,grok,...`.
- **`--dump-dom` + `--dump-out <path>` + `--dump-settle-secs <N>`** —
  bypass extraction, capture rich DOM diagnostics (top tag/class/data-
  attr counts, candidate conversation roots via repeating-sibling
  heuristic, top-root outerHTML truncated). Used to bootstrap selectors
  for new providers.
- **Per-provider extraction scripts**: Gemini wired (handles legacy
  `<user-query>`/`<model-response>` and current `#chat-history` +
  `<message-content>` + `.markdown.markdown-main-panel` shapes).
  ChatGPT, Claude, Perplexity, Grok stubbed (`WAIT_STUB` /
  `EXTRACT_STUB` constants); fail-fast guard fires before launch when
  selectors are stub.
- **Landing-page CF wait**: after cookie reload, polls
  `detect_interstitial` for up to ~12s in 1.5–2.5s human-jitter steps
  before navigating to the conversation URL. Falls through with
  warning.
- **Stealth bootstrap**: `--disable-blink-features=AutomationControlled`,
  `disable_default_args()` (drops `--enable-automation`), fixed
  1440×900 viewport, init script overrides for `navigator.webdriver`,
  `window.chrome`, `navigator.languages`, `navigator.plugins`.
- **Interstitial guard** runs after every navigation; surfaces precise
  errors (Cloudflare challenge / bot challenge / auth required).

### `ask-perplexity-cli` features
- **Salvage on timeout** (Patch A, 2026-04-27): when
  `wait_for_answer_stable` hits the deep-research timeout, the
  function captures `page.content()` anyway and returns it for the
  parser to extract whatever rendered. Eliminates the silent-research-
  loss class of failure that bit the previous session.
- Existing: `--deep`, `--thread <slug>`, `--focus`, model selection,
  drunk-typist humanised input, mouse-jitter clicks, `sanitize_for_typing`
  Unicode ASCII fallback (broader than ask-grok-cli's set).

### `ask-grok-cli` features
- Browser bootstrap with stealth flags (limited compared to perplexity).
- Drunk-typist typing with typo simulation.
- **Known gap**: no Unicode sanitisation (em-dashes etc. fail with
  "Key not found"), no watchdog timer (can hang indefinitely after
  "Response container visible"). See roadmap.

---

## Roadmap

Ordered by priority. Each item is anchored to specific failure evidence
from the 2026-04-27 session and the prior-art research.

### Tier 1 — ship next

#### G1. Migrate `ask-perplexity-cli` and `ask-grok-cli` onto `ghostroute-browser`
**Why:** the shared crate exists and works. Both CLIs still maintain
their own divergent copies of stealth, human-pause, cookie-loading. The
em-dash bug in ask-grok-cli is the canonical example — adding the
sanitiser there meant duplicating ask-perplexity's work; one shared
import would have prevented the whole class.

**Scope:** ~1 day per CLI. Replace `browser/`, `automation/human.rs`,
`config/cookies.rs` with imports from `ghostroute-browser`. Ship per-CLI;
don't block on workspace-wide refactor.

#### G2. Watchdog primitive in `ghostroute-browser`
**Why:** ask-grok-cli hung 14 min after `Response container visible:
856926ms` in this session. Patch A's salvage path for ask-perplexity-cli
is the same shape generalised. 4-source consensus from the post-mortem.

**Scope:** ~150 LOC. New `pub async fn with_watchdog<F, T>(timeout:
Duration, salvage_path: F) -> Result<(T, Option<TimeoutErr>)>` that runs
F with a hard deadline; on timeout calls a salvage closure. Apply to
both CLIs' wait-for-answer paths.

#### G3. Fix `chrome_appears_running` symlink check
**Why:** `Path::exists()` follows symlinks; chromium's `SingletonLock`
is a symlink to a stale `<host>-<pid>` target that's unresolvable, so
exists() returns false and the guard misses real lock state. Cost us
manual `rm -rf` interventions twice this session.

**Scope:** ~10 LOC. Use `fs::symlink_metadata().is_ok()` in
`fast-travel-cli/src/main.rs::chrome_appears_running`. Optionally read
the symlink target, parse `<host>-<pid>`, check if the PID is alive
via `/proc/<pid>` or `kill -0` for a stale-vs-active distinction.

#### G4. ASCII normalisation as a transport-layer concern
**Why:** session repeatedly broke on em-dash (`—`), multiplication sign
(`×`), greater-than-or-equal (`≥`). Each CLI has a different fragmented
mapping table. 4-source consensus to make this a `ContextProvider`-shaped
normalisation layer with reversible mapping.

**Scope:** already exists as `sanitize_unicode_for_typing` in
`ghostroute-browser/src/human.rs` — adopt in both CLIs' typing paths
during G1 migration. Extend table to cover the operator-symbol set we
hit (×, ≥, ≤, →, ←).

### Tier 2 — v0.2

#### G5. Per-provider selector profiles in YAML / TOML registry
**Why:** Gemini DOM moved mid-session from `<user-query>` /
`<model-response>` to `#chat-history` + `<message-content>` +
`.markdown.markdown-main-panel`. Selectors as code constants made the
fix a `main.rs` edit + rebuild instead of a profile update. 4-source
prior-art consensus.

**Scope:** new module `ghostroute-browser::selectors` reading from
`~/.config/ghostroute/selectors/<provider>.toml`. Each profile carries
`version`, `wait_selector`, `extract_steps`, `last_success_at`,
`fallbacks`. CLI builds a registry at startup; per-call uses the
profile; on extraction failure tries fallbacks in order.

#### G6. Auto-selector discovery (heuristic + interactive)
**Why:** the `--dump-dom` mode already produces the heuristic input
(repeating-sibling pattern, depth scoring). One step beyond is
emitting a candidate `wait_selector` / `extract_steps` directly from the
dump and writing it to G5's registry as a draft. Combined with a
point-and-click `--discover` interactive mode for tie-break cases.

**Scope:** ~300 LOC heuristic + ~200 LOC interactive (later). Heuristic
v1 is straight from the prior-art research (longest sibling run × depth
scoring; we already implement the input side).

#### G7. Dual-read extraction (DOM + network-idle + OCR fallback)
**Why:** Perplexity ep2's "visible but not extracted" pattern. The
salvage path handles the timeout; this prevents it. Three independent
content-ready signals: DOM contains expected selector, network has been
idle ≥1s, screenshot OCR finds expected text snippets.

**Scope:** medium — DOM + network-idle are cheap; OCR via tesseract or
a small ML model is the heavier add. Ship DOM + network-idle first as
a "extraction-ready signal" combinator; OCR is later if needed.

### Tier 3 — v0.3 / structural

#### G8. Hybrid network-layer + browser-layer architecture
**Why:** prior-art research's #1 finding. `rquest`
(`github.com/0x676e67/rquest`) does TLS/JA3/JA4/HTTP2 fingerprint
impersonation in pure Rust. For non-JS-challenge HTTP, `rquest` lets us
bypass Cloudflare entirely *without* chromiumoxide — the request looks
like real Chrome at the protocol layer, not just the JS layer. Collapses
S6 (browserd daemon) and parts of S8 (cookie clearance handshake) into
a thinner library wrapper.

**Scope:** medium-large. Add `rquest` as an optional path; existing
chromiumoxide stays for actual browser-required work.
`ghostroute-browser` grows a `Provider::execution_mode()` enum with
`{Network, Browser, Hybrid}`. Routes requests to the cheap path
where possible.

#### G9. Cookie pre-seeding via `rookie`
**Why:** `cookie-master-key` Chrome extension doesn't always export
HttpOnly+Secure cookies (`cf_clearance` was missing in this session).
`rookie` (`github.com/thewh1teagle/rookie`) handles cross-platform
Chrome cookie extraction with master-key decryption — direct Rust dep,
solves the export gap.

**Scope:** small-medium. New `cookies::pre_seed_from_chrome(host) ->
Vec<CookieParam>` that decrypts the local Chrome SQLite Cookies file
and converts to CDP shape. Used as a bootstrap step before the existing
cookie-file path.

#### G10. ModelClient compatibility matrix (single-source-of-truth)
**Why:** session burned ~5 model attempts on codex (`o3`, `gpt-4o`,
`gpt-5`, `gpt-5-codex`, `codex-mini-latest`) before discovering the
ChatGPT-account auth tier whitelist. A 1-token "say hello" probe at
session start, cached 24h per `(auth_mode, model)`, would have
eliminated the loop in <1 second.

**Scope:** small. Lives in BorAI Spore (`spore-providers`); referenced
here because the same primitive serves any Rust agent CLI talking to
multi-tier auth providers.

### Tier 4 — adopt-then-replace

#### G11. Replace hand-rolled stealth with `chromiumoxide_stealth`
**Why:** `cloei/chromiumoxide_stealth` is a maintained Rust analogue of
selenium-stealth / puppeteer-extra-stealth specifically for
chromiumoxide. Has more battle-tested overrides than our hand-rolled
script (WebGL, audio context, permissions stubs, plugin shape).

**Scope:** small (drop-in replacement of `STEALTH_INIT_SCRIPT` import).
Verify in a branch; fall back to ours if behaviour regresses.

---

## Known issues

These are not roadmap items per se — they're working-as-designed gaps
worth flagging:

1. **Cloudflare on Perplexity headless**. Even with the LLM Chrome
   profile, headless chromiumoxide doesn't have the TLS fingerprint to
   pass CF's interactive challenge for direct conversation URLs. The
   landing-page poll (Patch B) helps but doesn't fix it. Real fix is G8
   (`rquest` hybrid). Today: use `--visible` and accept a Chromium
   window flash, or solve once interactively in `--init-all` and rely
   on the persistent profile's `cf_clearance` for short-lived
   subsequent runs.

2. **ask-grok-cli's hang on "Response container visible"**. No
   internal watchdog; we depend on the wrapper-Bash timeout instead.
   G2 fixes this.

3. **`--init-all` orphan-Singleton state**. If the visible Chromium
   from `--init-all` exits uncleanly (signal kill, parent-process
   death), the user-data-dir keeps stale `Singleton{Lock,Cookie,Socket}`
   files, blocking the next launch. G3's symlink-aware lock check plus
   an explicit cleanup path on launch would close this.

4. **No upstream telemetry**. Each CLI prints to stderr. Aggregating
   per-call stats (`tokens_in/out`, `wall_ms`, `cf_seen`, `cookies_used`)
   would help with both debugging and the BorAI Spore telemetry
   integration. Defer to v0.2.

---

## Versioning

All four crates are at `0.1.0`. Recommend bumping `ghostroute-browser`
to `0.1.1` after G1–G4 land and the consumer CLIs catch up. No public
release planned before `0.2.0`; current stance is
"pre-release, breaking changes expected".

---

## Pointers

- BorAI Spore (downstream consumer / target system):
  `~/code/build-in-public/docs/superpowers/specs/2026-04-27-borai-spore-design.md`
- Spore feature list (covers more than ghostroute):
  `~/code/build-in-public/docs/superpowers/feature-list.md`
- Prior-art research (informs G5 / G7 / G8 / G9 / G11):
  `~/code/build-in-public/docs/superpowers/ghostroute-prior-art-2026-04-27.md`
- Session post-mortem (anchors most "Why" lines above):
  `~/code/build-in-public/docs/superpowers/session-analysis-2026-04-27.md`
