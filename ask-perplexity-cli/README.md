# ask-perplexity-cli

A native Rust CLI for [perplexity.ai](https://perplexity.ai), built on
`chromiumoxide`. Sibling to `ask-grok-cli`. Produces the same JSON output
shape as the Node provider's `/ask-perplexity` endpoint.

## Requirements

- Rust 2024 edition (stable)
- Valid Perplexity Pro cookies exported to
  `~/.claude/cookie-configs/perplexity.ai-cookies.json`
  (use the `cookie-master-key` browser extension in this repo)
- A display (X or Wayland) — the scraper runs headed Chromium because
  Cloudflare detects the `headless` flag. On headless servers, run under
  `Xvfb`.

## Install

```bash
cd ask-perplexity-cli
cargo install --path .
```

Or run directly from the workspace:

```bash
cargo run --release -- "your prompt here"
```

## Usage

```bash
# Basic query (Best model, web focus)
ask-perplexity "what is the capital of Australia?"

# Specific model
ask-perplexity --model claude "explain Kleisli composition"

# Academic focus (entry URL = /academic)
ask-perplexity --focus academic "recent advances in topological insulators"

# Deep Research (synchronous; blocks up to 30 min)
ask-perplexity --deep "comprehensive report on room-temperature superconductors"

# Continue an existing thread
ask-perplexity --thread abc-123-uuid "follow-up question"

# Include raw HTML blobs (for debugging or downstream parsing)
ask-perplexity --raw "a question"
```

## Output

JSON to stdout. Progress and diagnostic lines (`--deep` only) go to stderr
so `| jq` works directly.

```json
{
  "answer": "...",
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
  },
  "jobId": "..."
}
```

Fields:

| Field      | Always? | Notes |
|---         |---      |--- |
| `answer`   | yes     | Final response prose |
| `sources`  | yes     | `[]` when nothing was browsed |
| `threadId` | yes     | `null` when the URL isn't a `/search/<slug>` page |
| `steps`    | `--deep` only | Each step's phase: identifying, searching, insights, or other |
| `raw`      | `--raw` only | Raw HTML blobs for answer + sources containers |
| `jobId`    | `--deep` only | Synthetic correlation ID echoed to stderr as `[job] <uuid>` |

## Notes on detection bypass

Perplexity gates several Pro features behind automation-detection
checks. The browser bootstrap:

1. Runs **headed** (Cloudflare detects the `headless` flag directly).
2. Suppresses `--enable-automation` in the Chromium launch args.
3. Passes `--disable-blink-features=AutomationControlled`.
4. Injects an init script overriding `navigator.webdriver`,
   `window.chrome`, and `navigator.languages`.
5. Uses human-behaviour helpers (typing jitter, occasional typos with
   corrections, mouse-move trails before clicks) as defence-in-depth.

Without the stealth flags, Perplexity serves a stub Model menu with
only "Sonar" visible; with them, all 7 models appear.

## Known limitations

- Deep Research's source panel is not currently captured (the Node
  provider has the same limitation — logged separately).
- Thread follow-ups sometimes return the original thread's first answer
  rather than the follow-up answer, depending on parse `.last()` behaviour.
- `focus=academic`/`finance`/`health`/`patents` hide the Model menu —
  passing `--model` on those routes is silently ignored and Perplexity's
  default for that topic is used.

## Related

- `providers/perplexity/` — Node HTTP provider (same scraping strategy,
  same fixtures).
- `docs/superpowers/specs/2026-04-23-perplexity-scraper-design.md` — the
  design spec shared by both implementations.
