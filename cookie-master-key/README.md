# cookie-master-key

Chrome extension that exports session cookies from `x.com` and `grok.com` in the
format consumed by the `ghostroute` scrapers.

## Install (unpacked)

1. Open `chrome://extensions` in Chrome or a Chromium-based browser.
2. Enable **Developer mode** (top right).
3. Click **Load unpacked** and select this directory (`cookie-master-key/`).
4. Pin the extension for convenience.

## Usage

1. Log in to `x.com` or `grok.com` in the browser.
2. Click the extension icon while on the site.
3. The exported cookies JSON is placed where the scrapers expect it (see the
   ghostroute root README and each scraper's README for the target path —
   typically `~/.claude/cookie-configs/<domain>-cookies.json`).

## Output format

The extension writes a JSON array of cookie objects matching the shape consumed
by Playwright / `chromiumoxide`: one object per cookie, with `name`, `value`,
`domain`, `path`, `secure`, `httpOnly`, `sameSite`, and `expires` fields.
