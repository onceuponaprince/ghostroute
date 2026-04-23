# cookie-master-key

Chrome extension that exports session cookies from any logged-in site in the
format consumed by ghostroute's providers and their Rust CLI siblings.

## Install (unpacked)

1. Open `chrome://extensions` in Chrome or a Chromium-based browser.
2. Enable **Developer mode** (top right).
3. Click **Load unpacked** and select this directory (`cookie-master-key/`).
4. Pin the extension for convenience.

## Usage

1. Log in, in Chrome, to the site whose cookies you want to export — for
   example `x.com`, `grok.com`, `perplexity.ai`, or `gemini.google.com`.
2. With that tab active, click the extension icon.
3. Click **Export**. Chrome's *Save As* dialogue opens so you can pick the
   destination, and the file is named `<hostname>-cookies.json`.
4. Move the file to `~/.claude/cookie-configs/` — the global directory every
   consumer reads from:

   ```bash
   mkdir -p ~/.claude/cookie-configs
   mv ~/Downloads/gemini.google.com-cookies.json ~/.claude/cookie-configs/
   ```

The extension's logic is hostname-agnostic — `popup.js` reads whatever domain
the active tab is on and exports its cookies. No manifest edits or code changes
are needed to support a new site.

## Output format

The extension writes a JSON array of cookie objects matching the shape consumed
by Playwright and `chromiumoxide`: one object per cookie with `name`, `value`,
`domain`, `path`, `secure`, `httpOnly`, `sameSite`, and `expires` fields.
Chrome's `sameSite` values are converted to the Playwright-compatible forms
(`no_restriction` → `None`, `lax` → `Lax`, everything else → `Strict`).

## Known consumers

Tools that read from `~/.claude/cookie-configs/<domain>-cookies.json`:

- `ghostroute/server.js` and `providers/*/` — Node scraping surfaces.
- `ghostroute/ask-grok-cli/` — Rust CLI for Grok.
- `fast-travel-cli` (separate repo) — Rust CLI for Gemini.

Future consumers drop in without changes to this extension, provided they read
from the same cookie directory and accept the Playwright cookie shape.
