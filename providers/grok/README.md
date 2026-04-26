# providers/grok

**State:** shipped, with one structural caveat — the Node-side Grok code lives at the repo root rather than inside this directory. The CLI sibling lives at [`../../ask-grok-cli/`](../../ask-grok-cli/) and follows the standard shape.

This directory exists as the documentation home for Grok-the-provider, so the `providers/<name>/` shape stays uniform as the monorepo grows. The actual provider files were written before the monorepo-of-providers pattern landed and have not yet been relocated. Naming the asymmetry here is the cheap fix; the relocation is a separate beat.

## Where the code actually lives

| File | Path | Role |
|------|------|------|
| `askGrok()` implementation | [`../../grok-reverse-api-grok-main.js`](../../grok-reverse-api-grok-main.js) | Playwright + stealth + cookie-reuse + human-paced typing. The function the HTTP server calls. |
| HTTP endpoint | [`../../server.js`](../../server.js) | Mounts `POST /ask-grok` and forwards `req.body.prompt` into `askGrok()`. |
| Earlier draft | [`../../grok-reverse-api.js`](../../grok-reverse-api.js) | Superseded by `*-grok-main.js`. Retained as historical record per `docs/architecture.md#node-file-archaeology`. |
| Rust CLI sibling | [`../../ask-grok-cli/`](../../ask-grok-cli/) | Standalone terminal client on `chromiumoxide`. Same target, different transport. |

## Contract

### Input

```js
askGrok(prompt: string) → Promise<string>
```

Single positional string argument. No options object yet — the function predates the structured-options shape Perplexity introduced.

### Output

A single string: Grok's final response text. The HTTP layer wraps it as `{ result }`. There is no `sources[]`, no `threadId`, no `steps[]` — Grok's surface does not expose those affordances in the form the provider uses.

### Transports

- **HTTP.** `POST /ask-grok` with `{ prompt }`. Returns `{ result }`. See [`../../docs/server.md`](../../docs/server.md) for endpoint details, error shapes, and port handling.
- **Shell.** `cd ask-grok-cli && cargo run --release -- --prompt "..."`. Standalone single-binary; does not depend on a running server. See [`../../ask-grok-cli/README.md`](../../ask-grok-cli/README.md).

Both transports read cookies from `~/.claude/cookie-configs/grok.com-cookies.json`. The Node side has a working-directory fallback to `./grok.com-cookies.json` for legacy reasons; new consumers should rely on the global path.

## Cookies

Export from an authenticated `grok.com` tab using `cookie-master-key`. Move the resulting `grok.com-cookies.json` into `~/.claude/cookie-configs/`. See [`../../cookie-master-key/README.md`](../../cookie-master-key/README.md).

The `x.com-cookies.json` file at the repo root is a leftover from when Grok was reached via x.com. It is not the current cookie source and can be ignored.

## Tests

The Grok surface has [`../../server.test.js`](../../server.test.js) covering the `/ask-grok` endpoint at the HTTP layer. There is no parse/scrape unit-test split for Grok — the shape pre-dates the layered Perplexity pattern. New work on Grok would be a good moment to introduce one.

## Known gaps

- **No `index.js` in this directory.** The Node implementation is at the repo root, not behind a `providers/grok/index.js` re-export. A future cleanup pass would move `askGrok()` here and have the root file re-export for backwards compatibility, mirroring the Perplexity layout.
- **No `__fixtures__/` for parse-layer testing.** The Grok scraper extracts text directly inside its `page.evaluate` rather than returning HTML to a separate `parse.js`. Splitting parse from scrape, the way Perplexity does, would let the parse layer be exercised against captured HTML without a browser.
- **Single-string input contract.** Other providers accept `{ prompt, ...options }`. Promoting Grok's signature to the same shape would let it accept future options (model selection if Grok exposes one, thread continuation) without a breaking change.

These are noted, not fixed — this PR is documentation only.

## Related

- [`../../README.md`](../../README.md) — repo-level README; Components and Providers sections list both Grok surfaces.
- [`../../docs/architecture.md`](../../docs/architecture.md) — why Node lives at the root and Rust lives in its own crate.
- [`../../docs/server.md`](../../docs/server.md) — the `/ask-grok` HTTP endpoint, port behaviour, error shapes.
- [`../../ask-grok-cli/README.md`](../../ask-grok-cli/README.md) — Rust CLI usage, memory conventions, Claude Code orchestration hook.
