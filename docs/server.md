# server.js

The Express reverse-API server. Exposes ghostroute's provider scrapers as HTTP endpoints so Claude Code — or any other HTTP-capable caller — can invoke them without embedding browser-automation logic.

Source: [`server.js`](../server.js) at the repo root.

---

## Purpose

The server is a thin shim between HTTP and the scraper libraries. It exists so that:

1. **Claude Code can call scrapers by URL** rather than by shelling out to a Node script. Claude's HTTP tool is more ergonomic than its bash tool for long-running async calls.
2. **Multiple callers can share a single browser session** if the scraper library supports it. A long-running server amortises the Chromium startup cost across requests.
3. **Provider logic stays decoupled from transport.** The scraper functions (`askGrok`, future `askPerplexity`, etc.) are plain async functions; the server wraps them in an HTTP envelope. If a different transport is ever useful (a local CLI calling it, a Claude-Code skill invoking it via MCP, a background job queue), the same functions can be reused.

---

## Starting

```bash
npm start
```

Runs `node server.js` per the `package.json` scripts field. No build step — the codebase is ESM (`"type": "module"`) and runs directly on Node 20+.

On start, the server logs:

```
🍺 The Tavern is open! Reverse API running on http://localhost:3005
```

(The game-metaphor phrasing reflects the repo's runtime-log convention, not its documentation register.)

---

## Port fallback

The server binds to `PORT` environment variable or `3005` as default. If the port is already in use (`EADDRINUSE`), it retries on the next port up, up to **10 attempts**. If ten consecutive ports are busy, the process exits with code 1 and a clear message.

```bash
PORT=4000 npm start        # bind to 4000, or 4001..4009 if busy
```

Rationale: local development sometimes has orphaned Node processes from earlier runs. Retrying avoids a manual `kill` before each restart. The cap of ten prevents runaway retries if something environmental has pinned a wide port range.

---

## Endpoints

### `POST /ask-grok`

Invokes `askGrok()` from [`grok-reverse-api-grok-main.js`](../grok-reverse-api-grok-main.js). The function opens a Playwright session, injects Grok cookies from the cookie-jar, types the prompt with human-paced delays, waits for a stable response, and returns the text.

**Request:**

```json
{
  "prompt": "what is the current block height on Ethereum?"
}
```

**Response (success):**

```json
{
  "result": "As of my last update..."
}
```

**Response (no prompt):**

```json
{
  "error": "No prompt provided"
}
```

HTTP 400.

**Response (scraper failure):**

```json
{
  "error": "Grok request failed",
  "details": "<error message>"
}
```

HTTP 500. Typical causes: expired cookies, Grok UI change that broke selectors, Chromium launch failure.

---

## Claude Code integration pattern

The server is designed to be invoked from Claude Code sessions as part of the delegate-agent routing work (see Scene 2a-04 of the Command Centre campaign). Two invocation shapes:

### Direct HTTP call

Claude issues an HTTP request via its built-in tool:

```
POST http://localhost:3005/ask-grok
Content-Type: application/json

{"prompt": "<delegated task>"}
```

The response body's `result` field is the Grok answer. Claude can then interpret, summarise, or use the result as context for its own next step.

### Orchestrated via MCP (future)

Planned evolution: expose the same endpoints as MCP tools so Claude can invoke them with the same ergonomics as its built-in tools, without needing to know the HTTP shape. Not yet implemented; tracked in the delegate-agent scenes.

---

## Running alongside other dev processes

The server runs on Node's event loop and holds a persistent Chromium process open for the duration of each scraper invocation. It is not intended to run as a background service yet — no process manager, no logging to disk, no health checks. Start it on demand (`npm start`), call it, stop it.

For background-service operation, the next steps would be:

- A structured logger (pino) instead of `console.log`.
- Graceful shutdown on SIGTERM/SIGINT that closes any open browser sessions.
- A health check endpoint.
- A process manager (pm2, systemd) or container wrapper.

These are not current scope. The server is a developer-workflow tool, not a production service.

---

## Environment variables

| Name | Default | Purpose |
|------|---------|---------|
| `PORT` | `3005` | Starting port for the server. If busy, retries on next port up to 10 tries. |

The scrapers the server imports may read their own environment — currently the `askGrok` function reads `./grok.com-cookies.json` from the working directory as a fallback, though the ghostroute convention is to use `~/.claude/cookie-configs/grok.com-cookies.json`. Consult each provider's documentation for its cookie-path conventions.

---

## Related

- [`../README.md`](../README.md) — root entry with component list and usage.
- [`architecture.md`](architecture.md) — why the Node side exists and what the `.js` files at the repo root are.
- [`../grok-reverse-api-grok-main.js`](../grok-reverse-api-grok-main.js) — the current `askGrok()` implementation the server imports.
