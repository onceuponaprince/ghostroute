# fast-travel-cli

Carry Gemini conversations into Claude without blowing the context budget.

`fast-travel-cli` is the Gemini sibling to [`ask-grok-cli`](../ask-grok-cli/). Both belong to the context-hygiene layer: side-LLM outputs are only useful to Claude if they can enter a Claude session without pasting entire transcripts. `ask-grok-cli` writes a prompt into Grok and returns the answer. `fast-travel-cli` inverts the direction — it reads a conversation the user already had with Gemini in another tab and emits it as markdown to stdout.

## First-run scope

- Reads a single Gemini conversation by URL.
- Extracts the whole conversation (no range selection yet).
- Emits markdown to stdout.
- Headless Chromium.

Out of scope for first-run: range selection, output format flags, visible-browser debugging mode, installed-binary packaging, typed error taxonomy, tests.

## Installation

From the repo root:

```
cargo build --release
```

The binary lands at `target/release/fast-travel-cli`. No `cargo install` yet — first-run is exploratory.

## Cookie setup

`fast-travel-cli` reuses an authenticated Gemini browser session instead of relying on an API key. Session-reuse scrapers survive credit exhaustion; API-key paths die when the credits die.

### 1. Install cookie-master-key

Load the Chrome extension at [`../cookie-master-key/`](../cookie-master-key/) as an unpacked extension (`chrome://extensions/` → enable Developer Mode → *Load unpacked*).

### 2. Export cookies from an authenticated Gemini tab

1. Log in to [gemini.google.com](https://gemini.google.com/) in Chrome.
2. With a Gemini conversation open in the active tab, click the cookie-master-key extension icon.
3. Click *Export cookies*. The extension writes to the configured output directory.

### 3. Place the export at the expected path

`fast-travel-cli` looks for the cookie file at:

```
~/.claude/cookie-configs/gemini.google.com-cookies.json
```

Move or copy the export there. The file is a JSON array of CDP-compatible cookie objects (`name`, `value`, `domain`, `path`, `expires`, `httpOnly`, `secure`, `sameSite`).

If the export contains only analytics cookies (e.g. `_ga`, `_gcl_au`) and no session cookies (`SID`, `SSID`, `__Secure-*`), the tool will hit an auth redirect on launch and surface a legible error. Re-export with session cookies included.

## Usage

```
fast-travel-cli --conversation-url https://gemini.google.com/app/<conversation-id>
```

Pipe to a file:

```
fast-travel-cli --conversation-url https://gemini.google.com/app/<id> > conversation.md
```

Feed directly to Claude Code:

```
fast-travel-cli --conversation-url https://gemini.google.com/app/<id> | claude --resume
```

### Output shape

```markdown
## User

<first user message>

## Model

<first model response>

## User

...
```

No frontmatter, no citations, no mode annotations.

## Known stalls

- **Auth redirect loop.** Stale or incomplete cookies redirect to Google auth. The tool detects this before waiting on the conversation DOM and surfaces a clear error pointing at the cookie re-export path.
- **Gemini DOM selector drift.** Gemini's UI updates frequently. Extraction selectors live in one `page.evaluate` string — when they drift, update that single block.
- **Virtualised rendering.** If Gemini lazily renders older messages, extraction may truncate silently. The tool scrolls to top once before extracting, which handles most cases.

## Architecture

Single-file `src/main.rs`. Will split into `browser/`, `automation/`, `cli/`, `config/` modules once the file crosses ~400 lines (the `ask-grok-cli` shape).

- `load_cookies` — reads the JSON cookie file from the global config path.
- `launch_browser` — boots headless Chromium via `chromiumoxide`.
- `inject_cookies_and_navigate` — navigates to `gemini.google.com` for domain context (CDP rejects `set_cookies` on `about:blank`), injects cookies, reloads, navigates to the conversation URL, detects auth redirects early.
- `wait_for_conversation` — polls for Gemini's conversation DOM with a ~30s timeout.
- `extract_conversation` — `page.evaluate` returns `{messages: [{role, content}]}`; Rust deserialises.
- `render_markdown` — stdout, role-prefixed headers.

## License

MIT.
