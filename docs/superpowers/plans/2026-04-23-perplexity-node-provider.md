# Perplexity Node provider — implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship a working `/ask-perplexity` HTTP surface (fast modes + Deep Research async job pattern + optional threading) backed by a `providers/perplexity/` module with parse-layer unit test coverage against captured HTML fixtures.

**Architecture:** Node provider module under `providers/perplexity/` exposes `askPerplexity()` (synchronous, fast modes) and `askPerplexityDeep()` (async job lifecycle). `server.js` adds three endpoints. Structured output with inline citation markers preserved; raw HTML escape hatch opt-in. In-memory job store. See spec: `docs/superpowers/specs/2026-04-23-perplexity-scraper-design.md`.

**Tech Stack:** Node.js (ESM), Playwright (browser), Cheerio (HTML parsing — already a dep), Vitest (test runner — to be added), Supertest (HTTP integration tests — to be added), Express 5.

**Operational note:** Tasks 2, 11–14, and 20 require a live Perplexity Pro session with valid cookies at `~/.claude/cookie-configs/perplexity.ai-cookies.json`. Tasks 3–9 and 15–19 run offline against fixtures / mocks. Smoke-test tasks are gated by the `SMOKE=1` env var so a bare `npm test` never hits the network.

---

## File operations summary

| Path | Operation |
| --- | --- |
| `package.json` | add `vitest`, `supertest` to devDependencies; update `test` and add `test:e2e:perplexity` scripts |
| `vitest.config.js` | create |
| `~/.claude/cookie-configs/perplexity.ai-cookies.json` | create manually (outside repo) |
| `providers/perplexity/__fixtures__/auto-web.html` | create (captured manually in Task 2) |
| `providers/perplexity/__fixtures__/pro-web.html` | create |
| `providers/perplexity/__fixtures__/reasoning-web.html` | create |
| `providers/perplexity/__fixtures__/deep-research-web.html` | create |
| `providers/perplexity/__fixtures__/writing-focus.html` | create |
| `providers/perplexity/errors.js` | create |
| `providers/perplexity/parse.js` | create |
| `providers/perplexity/selectors.js` | create |
| `providers/perplexity/scrape.js` | create |
| `providers/perplexity/jobs.js` | create |
| `providers/perplexity/index.js` | create |
| `providers/perplexity/*.test.js` | create (one test file per module) |
| `server.js` | modify — add 3 Perplexity endpoints |
| `README.md` | modify — document Perplexity surface |

---

## Task 1: Scaffold test tooling + provider directory

**Files:**
- Modify: `package.json`
- Create: `vitest.config.js`
- Create: `providers/perplexity/` (directory)
- Create: `providers/perplexity/__fixtures__/` (directory)

- [ ] **Step 1: Install vitest and supertest**

Run:
```bash
npm install -D vitest supertest
```

Expected: `devDependencies` updated in `package.json`; `package-lock.json` regenerated.

- [ ] **Step 2: Update `package.json` scripts**

Open `package.json`. Replace the `scripts` block with:

```json
  "scripts": {
    "start": "node server.js",
    "ask": "node grok-reverse-api-grok-main.js",
    "test": "vitest run",
    "test:watch": "vitest",
    "test:e2e:perplexity": "SMOKE=1 vitest run providers/perplexity/scrape.test.js"
  },
```

- [ ] **Step 3: Create `vitest.config.js`**

Create `vitest.config.js` at the repo root:

```js
import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    include: ['providers/**/*.test.js'],
    // Smoke tests opt in via SMOKE=1 env var; they check this at runtime and skip otherwise.
    testTimeout: 60_000,
    hookTimeout: 60_000,
  },
});
```

- [ ] **Step 4: Create provider directories**

Run:
```bash
mkdir -p providers/perplexity/__fixtures__
```

- [ ] **Step 5: Verify vitest runs (and finds no tests yet)**

Run:
```bash
npm test
```

Expected: output includes `No test files found` (or similar) and exit code 0 or 1 — either is fine, we just need to confirm vitest is callable.

- [ ] **Step 6: Commit**

```bash
git add package.json package-lock.json vitest.config.js providers/ 2>/dev/null
git commit -m "chore: scaffold vitest + providers/perplexity/ directory"
```

(Whichever lockfile exists will be staged; the `2>/dev/null` hides the error for the absent one.)

---

## Task 2: Capture HTML fixtures from a live Perplexity Pro session

**Files:**
- Create: `~/.claude/cookie-configs/perplexity.ai-cookies.json`
- Create: `providers/perplexity/__fixtures__/auto-web.html`
- Create: `providers/perplexity/__fixtures__/pro-web.html`
- Create: `providers/perplexity/__fixtures__/reasoning-web.html`
- Create: `providers/perplexity/__fixtures__/deep-research-web.html`
- Create: `providers/perplexity/__fixtures__/writing-focus.html`

This is a manual capture step. Every parse test downstream depends on these files. Use the same trivial prompt for every capture so answers are predictable for content assertions: **`"who founded meta (formerly facebook)?"`**. Answer should reference Zuckerberg in every mode.

- [ ] **Step 1: Export Perplexity cookies**

Install the `cookie-master-key/` Chrome extension (see its README), log into perplexity.ai with the Pro account in Chrome, click the extension icon, and export `perplexity.ai` cookies as JSON.

Save to: `~/.claude/cookie-configs/perplexity.ai-cookies.json`

Run to verify:
```bash
test -f ~/.claude/cookie-configs/perplexity.ai-cookies.json && echo "ok"
```

Expected: `ok`

- [ ] **Step 2: Capture `auto-web.html` via Playwright codegen**

Run:
```bash
npx playwright codegen https://perplexity.ai
```

In the opened browser:
1. Open DevTools → Application → Cookies → paste cookies from the JSON file (or load the session via extension)
2. Set mode to **Auto**, focus to **Web**
3. Type the prompt `who founded meta (formerly facebook)?` and submit
4. Wait for the full response with sources to load (~30s)
5. In DevTools → Elements, right-click `<html>` → Copy → Copy outerHTML
6. Save to `providers/perplexity/__fixtures__/auto-web.html`

While codegen is open, **write down the selectors you observed for**:
- mode selector button / dropdown
- focus filter button / dropdown
- prompt input (`contenteditable` div? `<textarea>`?)
- submit button OR keyboard shortcut
- answer container
- sources container
- individual source item (title, url, snippet)
- inline citation anchor
- thread URL pattern (should be `/search/<uuid>`)

These go into `selectors.js` in Task 10. Keep notes in a scratch file — do not commit it.

- [ ] **Step 3: Capture the other four fixtures**

Repeat Step 2 for each remaining fixture. Same prompt every time.

| Fixture | Mode | Focus |
| --- | --- | --- |
| `pro-web.html` | Pro | Web |
| `reasoning-web.html` | Reasoning | Web |
| `deep-research-web.html` | Deep Research | Web |
| `writing-focus.html` | Auto | Writing |

For `deep-research-web.html`, wait for the full Deep Research run to complete (~5 min). It should include the "steps" panel showing intermediate searches.

For `writing-focus.html`, there will be no sources (Writing mode is LLM-only).

- [ ] **Step 4: Verify fixtures exist and are non-trivial**

Run:
```bash
ls -la providers/perplexity/__fixtures__/
wc -c providers/perplexity/__fixtures__/*.html
```

Expected: five files, each >10KB (Perplexity responses are heavyweight).

- [ ] **Step 5: Commit fixtures**

```bash
git add providers/perplexity/__fixtures__/
git commit -m "test(perplexity): add captured HTML fixtures for parse tests"
```

---

## Task 3: `errors.js` — typed error classes

**Files:**
- Create: `providers/perplexity/errors.js`
- Create: `providers/perplexity/errors.test.js`

- [ ] **Step 1: Write failing test**

Create `providers/perplexity/errors.test.js`:

```js
import { describe, it, expect } from 'vitest';
import {
  PerplexityAuthError,
  PerplexityScrapeError,
  PerplexityTimeoutError,
  PerplexityParseError,
} from './errors.js';

describe('perplexity errors', () => {
  it('PerplexityAuthError carries the refresh-cookies message', () => {
    const err = new PerplexityAuthError();
    expect(err).toBeInstanceOf(Error);
    expect(err.name).toBe('PerplexityAuthError');
    expect(err.message).toMatch(/refresh cookies/i);
  });

  it('PerplexityScrapeError captures stage, selector, html', () => {
    const err = new PerplexityScrapeError('await-completion', 'div.done', '<html>...</html>');
    expect(err.name).toBe('PerplexityScrapeError');
    expect(err.stage).toBe('await-completion');
    expect(err.selector).toBe('div.done');
    expect(err.html).toBe('<html>...</html>');
  });

  it('PerplexityTimeoutError captures the stage that timed out', () => {
    const err = new PerplexityTimeoutError('await-completion', 180_000);
    expect(err.name).toBe('PerplexityTimeoutError');
    expect(err.stage).toBe('await-completion');
    expect(err.timeoutMs).toBe(180_000);
  });

  it('PerplexityParseError carries the original html for debugging', () => {
    const err = new PerplexityParseError('missing answer node', '<div></div>');
    expect(err.name).toBe('PerplexityParseError');
    expect(err.html).toBe('<div></div>');
  });

  it('Scrape error truncates html to 4KB for server logs', () => {
    const big = 'x'.repeat(10_000);
    const err = new PerplexityScrapeError('stage', 'sel', big);
    expect(err.htmlTruncated.length).toBeLessThanOrEqual(4096);
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run:
```bash
npm test providers/perplexity/errors.test.js
```

Expected: FAIL — module `./errors.js` does not exist.

- [ ] **Step 3: Implement `errors.js`**

Create `providers/perplexity/errors.js`:

```js
export class PerplexityAuthError extends Error {
  constructor() {
    super('Perplexity login wall hit — refresh cookies at ~/.claude/cookie-configs/perplexity.ai-cookies.json');
    this.name = 'PerplexityAuthError';
  }
}

export class PerplexityScrapeError extends Error {
  constructor(stage, selector, html) {
    super(`Perplexity scrape failed at stage "${stage}" (selector: ${selector})`);
    this.name = 'PerplexityScrapeError';
    this.stage = stage;
    this.selector = selector;
    this.html = html;
    this.htmlTruncated = (html || '').slice(0, 4096);
  }
}

export class PerplexityTimeoutError extends Error {
  constructor(stage, timeoutMs) {
    super(`Perplexity timed out at stage "${stage}" after ${timeoutMs}ms`);
    this.name = 'PerplexityTimeoutError';
    this.stage = stage;
    this.timeoutMs = timeoutMs;
  }
}

export class PerplexityParseError extends Error {
  constructor(reason, html) {
    super(`Perplexity parse failed: ${reason}`);
    this.name = 'PerplexityParseError';
    this.reason = reason;
    this.html = html;
  }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run:
```bash
npm test providers/perplexity/errors.test.js
```

Expected: PASS — 5 tests passing.

- [ ] **Step 5: Commit**

```bash
git add providers/perplexity/errors.js providers/perplexity/errors.test.js
git commit -m "feat(perplexity): add typed error classes"
```

---

## Task 4: `parse.js` — skeleton + answer extraction

**Files:**
- Create: `providers/perplexity/parse.js`
- Create: `providers/perplexity/parse.test.js`

- [ ] **Step 1: Write failing test**

Create `providers/perplexity/parse.test.js`:

```js
import { describe, it, expect } from 'vitest';
import { readFileSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { parse } from './parse.js';

const __dirname = dirname(fileURLToPath(import.meta.url));
const fixture = (name) =>
  readFileSync(resolve(__dirname, '__fixtures__', name), 'utf8');

describe('parse — answer extraction', () => {
  it('extracts a non-empty answer from auto-web fixture', () => {
    const result = parse(fixture('auto-web.html'), { url: 'https://perplexity.ai/search/abc123' });
    expect(typeof result.answer).toBe('string');
    expect(result.answer.length).toBeGreaterThan(20);
  });

  it('answer references Meta/Facebook founder for the seed prompt', () => {
    const result = parse(fixture('auto-web.html'), { url: 'https://perplexity.ai/search/abc123' });
    const answer = result.answer.toLowerCase();
    expect(answer).toMatch(/zuckerberg|meta|facebook/);
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run:
```bash
npm test providers/perplexity/parse.test.js
```

Expected: FAIL — module `./parse.js` does not exist.

- [ ] **Step 3: Implement minimal `parse.js`**

Create `providers/perplexity/parse.js`. This task implements answer extraction only. Sources, steps, raw, and threadId land in later tasks.

The selector below is a **placeholder-that-must-be-replaced in Task 10** once real selectors are captured. Use whatever matches the answer container in the captured fixtures as of fixture-capture date.

```js
import * as cheerio from 'cheerio';
import { PerplexityParseError } from './errors.js';
import { SELECTORS } from './selectors.js';

export function parse(html, { url } = {}) {
  const $ = cheerio.load(html);

  const answerNode = $(SELECTORS.answerContainer).last();
  if (answerNode.length === 0) {
    throw new PerplexityParseError('answer container not found', html);
  }
  const answer = answerNode.text().trim();
  if (!answer) {
    throw new PerplexityParseError('answer container empty', html);
  }

  return {
    answer,
    sources: [],
    threadId: null,
  };
}
```

- [ ] **Step 4: Create `selectors.js` stub so import resolves**

Create `providers/perplexity/selectors.js` with a minimal stub. The full file lands in Task 10.

```js
// Perplexity DOM selectors — the fragile layer.
// Prefer data-testid and aria-label over class names.
// Update here (and only here) when Perplexity ships a UI change.
export const SELECTORS = {
  answerContainer: 'div.prose', // TODO(Task 10): replace with real selector from captured fixtures
};
```

- [ ] **Step 5: Run test to verify it passes**

Run:
```bash
npm test providers/perplexity/parse.test.js
```

Expected: PASS — 2 tests passing.

If it fails because `div.prose` doesn't match the captured fixture: inspect the fixture HTML, find the answer container's actual selector, update `selectors.js`, re-run. This is the normal flow.

- [ ] **Step 6: Commit**

```bash
git add providers/perplexity/parse.js providers/perplexity/parse.test.js providers/perplexity/selectors.js
git commit -m "feat(perplexity): parse extracts answer from fixture"
```

---

## Task 5: `parse.js` — sources extraction

**Files:**
- Modify: `providers/perplexity/parse.js`
- Modify: `providers/perplexity/parse.test.js`

(`selectors.js` was populated in Task 10 with real values — `sourceItem`,
`sourceTitle`, `sourceSnippet` are already in place. Do not modify it.)

- [ ] **Step 1: Extend test file with sources assertions**

Append to `providers/perplexity/parse.test.js`:

```js
describe('parse — sources extraction', () => {
  it('extracts at least one source from auto-web fixture', () => {
    const result = parse(fixture('auto-web.html'), { url: 'https://perplexity.ai/search/abc' });
    expect(Array.isArray(result.sources)).toBe(true);
    expect(result.sources.length).toBeGreaterThanOrEqual(1);
  });

  it('each source has index, title, url, domain', () => {
    const result = parse(fixture('auto-web.html'), { url: 'https://perplexity.ai/search/abc' });
    for (const source of result.sources) {
      expect(typeof source.index).toBe('number');
      expect(typeof source.title).toBe('string');
      expect(source.url).toMatch(/^https?:\/\//);
      expect(source.domain).toMatch(/\./);
    }
  });

  it('source indices are 1-based and sequential', () => {
    const result = parse(fixture('auto-web.html'), { url: 'https://perplexity.ai/search/abc' });
    result.sources.forEach((s, i) => {
      expect(s.index).toBe(i + 1);
    });
  });
});
```

- [ ] **Step 2: Run tests to verify new ones fail**

Run:
```bash
npm test providers/perplexity/parse.test.js
```

Expected: 3 new tests FAIL (sources always empty from Task 4); original 2 still PASS.

- [ ] **Step 3: Implement sources extraction in `parse.js`**

Sources in Perplexity's DOM: each source card is an `<a href>` whose own
href is the source URL. Inside it are two spans — title and snippet —
identifiable by Tailwind utility classes. Replace `parse.js` with:

```js
import * as cheerio from 'cheerio';
import { PerplexityParseError } from './errors.js';
import { SELECTORS } from './selectors.js';

export function parse(html, { url } = {}) {
  const $ = cheerio.load(html);

  const answerNode = $(SELECTORS.answerContainer).last();
  if (answerNode.length === 0) {
    throw new PerplexityParseError('answer container not found', html);
  }
  const answer = answerNode.text().trim();
  if (!answer) {
    throw new PerplexityParseError('answer container empty', html);
  }

  const sources = extractSources($);

  return {
    answer,
    sources,
    threadId: null,
  };
}

function extractSources($) {
  const items = $(SELECTORS.sourceItem);
  if (items.length === 0) return [];

  const out = [];
  items.each((i, el) => {
    const $el = $(el);
    const href = $el.attr('href');
    if (!href) return;
    let domain = '';
    try {
      domain = new URL(href).hostname;
    } catch {
      return;
    }
    const title = $el.find(SELECTORS.sourceTitle).first().text().trim() || domain;
    const snippet = $el.find(SELECTORS.sourceSnippet).first().text().trim() || undefined;
    out.push({ index: i + 1, title, url: href, domain, snippet });
  });
  return out;
}
```

Note: `sourceItem` is already the `<a>` tag (see Task 10 selectors —
`[class*="group/search-side-content"] a[href]`). Extract its own `href`
directly; no nested URL lookup is needed.

- [ ] **Step 4: Run tests — all 5 should pass**

Run:
```bash
npm test providers/perplexity/parse.test.js
```

Expected: 5 tests PASS (2 from Task 4 + 3 new).

- [ ] **Step 6: Commit**

```bash
git add providers/perplexity/parse.js providers/perplexity/parse.test.js providers/perplexity/selectors.js
git commit -m "feat(perplexity): parse extracts sources array"
```

---

## Task 6: DELETED — inline citation markers not applicable

Deleted 2026-04-24 after offline dissection of captured fixtures
(auto-web, reasoning-web, deep-research-web) via
`scripts/dissect-fixtures.mjs` found zero inline citation markers in
Perplexity's answer prose (no `[N]` text, no `<a>`, no `<sup>`). The
UI renders the answer as pure prose with sources listed separately.
Spec amended under §Revisions. Skip to Task 7.

No code work required. Task 7 below remains Task 7 — numbering preserved
to avoid churning cross-references in PROGRESS.md and TaskCreate IDs.

---

## Task 7: `parse.js` — Deep Research steps extraction

**Files:**
- Modify: `providers/perplexity/parse.test.js`
- Modify: `providers/perplexity/parse.js`

(`selectors.js` already contains `stepItem`, `stepQuery`, `stepPhaseIcon` and
exports `PHASE_BY_ICON` — populated in Task 10. Do not modify it.)

- [ ] **Step 1: Add failing tests for steps**

Append to `providers/perplexity/parse.test.js`:

```js
describe('parse — Deep Research steps', () => {
  it('extracts steps[] from deep-research fixture', () => {
    const result = parse(fixture('deep-research-web.html'), {
      url: 'https://perplexity.ai/search/xyz',
      mode: 'deep-research',
    });
    expect(Array.isArray(result.steps)).toBe(true);
    expect(result.steps.length).toBeGreaterThanOrEqual(1);
    const validPhases = ['identifying', 'searching', 'insights', 'other'];
    for (const step of result.steps) {
      expect(typeof step.query).toBe('string');
      expect(validPhases).toContain(step.phase);
    }
  });

  it('non-deep-research modes return no steps field', () => {
    const result = parse(fixture('auto-web.html'), { url: 'https://perplexity.ai/search/abc' });
    expect(result.steps).toBeUndefined();
  });
});
```

- [ ] **Step 2: Run — expect first new test to fail**

Run:
```bash
npm test providers/perplexity/parse.test.js
```

Expected: the `extracts steps[] from deep-research fixture` test FAILS (no steps field yet). The `non-deep-research modes return no steps field` test PASSES (no steps field = undefined = correct).

- [ ] **Step 3: Implement steps extraction in `parse.js`**

Replace the entire contents of `providers/perplexity/parse.js` with:

```js
import * as cheerio from 'cheerio';
import { PerplexityParseError } from './errors.js';
import { SELECTORS, PHASE_BY_ICON } from './selectors.js';

export function parse(html, { url, mode } = {}) {
  const $ = cheerio.load(html);

  const answerNode = $(SELECTORS.answerContainer).last();
  if (answerNode.length === 0) {
    throw new PerplexityParseError('answer container not found', html);
  }
  const answer = answerNode.text().trim();
  if (!answer) {
    throw new PerplexityParseError('answer container empty', html);
  }

  const sources = extractSources($);
  const result = { answer, sources, threadId: null };

  if (mode === 'deep-research') {
    result.steps = extractSteps($);
  }

  return result;
}

function extractSources($) {
  const items = $(SELECTORS.sourceItem);
  if (items.length === 0) return [];

  const out = [];
  items.each((i, el) => {
    const $el = $(el);
    const href = $el.attr('href');
    if (!href) return;
    let domain = '';
    try {
      domain = new URL(href).hostname;
    } catch {
      return;
    }
    const title = $el.find(SELECTORS.sourceTitle).first().text().trim() || domain;
    const snippet = $el.find(SELECTORS.sourceSnippet).first().text().trim() || undefined;
    out.push({ index: i + 1, title, url: href, domain, snippet });
  });
  return out;
}

function extractSteps($) {
  const items = $(SELECTORS.stepItem);
  const out = [];
  items.each((_, el) => {
    const $el = $(el);
    const query = $el.find(SELECTORS.stepQuery).first().text().trim();
    const iconEl = $el.find(SELECTORS.stepPhaseIcon).first();
    const iconRef = iconEl.attr('xlink:href') || iconEl.attr('href');
    const phase = PHASE_BY_ICON[iconRef] || 'other';
    if (query) out.push({ query, phase });
  });
  return out;
}
```

Note: `PHASE_BY_ICON` is imported from `selectors.js` (already exported there);
do not define a local copy in `parse.js`. The `iconEl` helper avoids calling
`.find(...)` twice.

- [ ] **Step 4: Run — all tests pass**

Run:
```bash
npm test providers/perplexity/parse.test.js
```

Expected: 7 tests PASS (2 from Task 4 + 3 from Task 5 + 2 new).

- [ ] **Step 5: Commit**

```bash
git add providers/perplexity/parse.js providers/perplexity/parse.test.js
git commit -m "feat(perplexity): parse extracts Deep Research steps"
```

---

## Task 8: `parse.js` — raw HTML escape hatch

**Files:**
- Modify: `providers/perplexity/parse.test.js`
- Modify: `providers/perplexity/parse.js`

- [ ] **Step 1: Add failing tests for raw option**

Append to `providers/perplexity/parse.test.js`:

```js
describe('parse — raw HTML escape hatch', () => {
  it('omits raw field by default', () => {
    const result = parse(fixture('auto-web.html'), { url: 'https://perplexity.ai/search/abc' });
    expect(result.raw).toBeUndefined();
  });

  it('includes raw.answerHtml and raw.sourcesHtml when raw: true', () => {
    const result = parse(fixture('auto-web.html'), {
      url: 'https://perplexity.ai/search/abc',
      raw: true,
    });
    expect(typeof result.raw.answerHtml).toBe('string');
    expect(result.raw.answerHtml.length).toBeGreaterThan(0);
    expect(typeof result.raw.sourcesHtml).toBe('string');
  });
});
```

- [ ] **Step 2: Run — expect failure**

Run:
```bash
npm test providers/perplexity/parse.test.js
```

Expected: new tests FAIL (no `raw` field).

- [ ] **Step 3: Implement raw support**

In `parse.js`, update the `parse` function body to optionally attach raw HTML:

```js
export function parse(html, { url, mode, raw = false } = {}) {
  const $ = cheerio.load(html);

  const answerNode = $(SELECTORS.answerContainer).last();
  if (answerNode.length === 0) {
    throw new PerplexityParseError('answer container not found', html);
  }
  const answerHtml = raw ? $.html(answerNode) : undefined;
  const answer = answerNode.text().trim();
  if (!answer) {
    throw new PerplexityParseError('answer container empty', html);
  }

  const sourcesNode = $(SELECTORS.sourcesContainer).first();
  const sourcesHtml = raw ? (sourcesNode.length ? $.html(sourcesNode) : '') : undefined;

  const sources = extractSources($);
  const result = { answer, sources, threadId: null };

  if (mode === 'deep-research') {
    result.steps = extractSteps($);
  }
  if (raw) {
    result.raw = { answerHtml, sourcesHtml };
  }
  return result;
}
```

- [ ] **Step 4: Run — all tests pass**

Run:
```bash
npm test providers/perplexity/parse.test.js
```

Expected: all tests PASS.

- [ ] **Step 5: Commit**

```bash
git add providers/perplexity/parse.js providers/perplexity/parse.test.js
git commit -m "feat(perplexity): parse supports raw HTML opt-in"
```

---

## Task 9: `parse.js` — threadId extraction from URL

**Files:**
- Modify: `providers/perplexity/parse.test.js`
- Modify: `providers/perplexity/parse.js`

- [ ] **Step 1: Add failing tests**

Append to `providers/perplexity/parse.test.js`:

```js
describe('parse — threadId', () => {
  it('extracts threadId from /search/<uuid> URL', () => {
    const result = parse(fixture('auto-web.html'), {
      url: 'https://www.perplexity.ai/search/abc-123-uuid',
    });
    expect(result.threadId).toBe('abc-123-uuid');
  });

  it('returns null threadId when URL lacks /search/ segment', () => {
    const result = parse(fixture('auto-web.html'), { url: 'https://www.perplexity.ai/' });
    expect(result.threadId).toBeNull();
  });

  it('returns null threadId when url option is omitted', () => {
    const result = parse(fixture('auto-web.html'));
    expect(result.threadId).toBeNull();
  });
});
```

- [ ] **Step 2: Run — expect failures**

Run:
```bash
npm test providers/perplexity/parse.test.js
```

Expected: new tests FAIL (threadId always null).

- [ ] **Step 3: Implement threadId extraction**

In `parse.js`, add near the top of the file (before `export function parse`):

```js
function extractThreadId(url) {
  if (!url) return null;
  try {
    const u = new URL(url);
    const m = u.pathname.match(/^\/search\/([^/?#]+)/);
    return m ? m[1] : null;
  } catch {
    return null;
  }
}
```

And in `parse()`, replace `threadId: null` with `threadId: extractThreadId(url)`.

- [ ] **Step 4: Run — all tests pass**

Run:
```bash
npm test providers/perplexity/parse.test.js
```

Expected: all tests PASS.

- [ ] **Step 5: Commit**

```bash
git add providers/perplexity/parse.js providers/perplexity/parse.test.js
git commit -m "feat(perplexity): parse extracts threadId from URL"
```

---

## Task 10: `selectors.js` — replace placeholder selectors with real ones

**Files:**
- Modify: `providers/perplexity/selectors.js`

This is a discovery task. Every placeholder selector added in Tasks 4–8 must now be replaced with a real selector captured from the live Perplexity UI and verified against the fixtures.

- [ ] **Step 1: Open each fixture in a browser**

Run:
```bash
xdg-open providers/perplexity/__fixtures__/auto-web.html
```

(Or drag into your browser.) Use DevTools to inspect and find:

| Logical name | What to look for |
| --- | --- |
| `answerContainer` | the `<div>` wrapping the assistant's answer prose |
| `sourcesContainer` | the `<div>` or `<section>` wrapping the source cards |
| `sourceItem` | each individual source card |
| `sourceTitle` | the source's title/headline text |
| `sourceUrl` | the source's link |
| `sourceSnippet` | the source's excerpt/snippet, if present |
| `citationAnchor` | each inline `[N]` citation link inside the answer |
| `stepItem` | (DR only) each step button |
| `stepQuery` | (DR only) the step description text inside the button |
| `stepPhaseIcon` | (DR only) the `<svg use>` inside the step button — read `xlink:href` |

**Prefer** `[data-testid="..."]` and `[aria-label="..."]` over class names. Class names are minified and change across deploys.

- [ ] **Step 2: Also identify selectors needed for `scrape.js` (Task 11+)**

These won't be asserted by parse tests but are needed for live scraping:

| Logical name | What to look for |
| --- | --- |
| `promptInput` | the text input where you type — likely `div[contenteditable="true"]` or `textarea` |
| `submitButton` | the send button (OR document that Enter key submits) |
| `modeButton` | the button/dropdown that opens mode selection |
| `modeOption(name)` | the option for each mode (pattern: accepts a mode name, returns its selector) |
| `focusButton` | the button/dropdown that opens focus filter selection |
| `focusOption(name)` | the option for each focus |
| `generatingIndicator` | the "Perplexity is thinking…" / spinner element (presence == still running) |
| `doneIndicator` | the element that only appears when generation is complete (presence == done). If absent, use `NOT generatingIndicator` logic. |
| `deepResearchProgressText` | the status text during a DR run (e.g., "Searching 12 sources") |
| `loginWallDetector` | the "Log in" button on the auth-required page |

- [ ] **Step 3: Rewrite `selectors.js` with real values**

Replace `providers/perplexity/selectors.js` with the complete structure below. Replace every `FILL-IN-TASK-10` string with the actual selector you captured. Keep the comment above each group explaining what it selects.

```js
// Perplexity DOM selectors — the fragile layer.
// Captured from live perplexity.ai on <date of capture>.
// Prefer data-testid and aria-label over class names.
// Update here (and only here) when Perplexity ships a UI change.

export const SELECTORS = {
  // --- parse layer (read from scraped HTML) ---
  answerContainer: 'FILL-IN-TASK-10',
  sourcesContainer: 'FILL-IN-TASK-10',
  sourceItem: 'FILL-IN-TASK-10',
  sourceTitle: 'FILL-IN-TASK-10',
  sourceUrl: 'FILL-IN-TASK-10',
  sourceSnippet: 'FILL-IN-TASK-10',
  citationAnchor: 'FILL-IN-TASK-10',
  stepItem: 'FILL-IN-TASK-10',
  stepQuery: 'FILL-IN-TASK-10',
  stepPhaseIcon: 'FILL-IN-TASK-10',

  // --- scrape layer (interact with live page) ---
  promptInput: 'FILL-IN-TASK-10',
  submitKey: 'Enter',                          // or adjust if Enter doesn't submit
  modeButton: 'FILL-IN-TASK-10',
  modeOption: (name) => `FILL-IN-TASK-10`,     // e.g. `button[data-mode="${name}"]`
  focusButton: 'FILL-IN-TASK-10',
  focusOption: (name) => `FILL-IN-TASK-10`,
  generatingIndicator: 'FILL-IN-TASK-10',
  doneIndicator: 'FILL-IN-TASK-10',            // use '' if you rely on absence of generatingIndicator instead
  deepResearchProgressText: 'FILL-IN-TASK-10',
  loginWallDetector: 'FILL-IN-TASK-10',
};
```

- [ ] **Step 4: Re-run ALL parse tests**

Run:
```bash
npm test providers/perplexity/parse.test.js
```

Expected: all tests from Tasks 4–9 PASS against real selectors. If any fail, the selector you captured doesn't match the fixture — adjust.

- [ ] **Step 5: Commit**

```bash
git add providers/perplexity/selectors.js
git commit -m "feat(perplexity): replace placeholder selectors with captured ones"
```

---

## Task 11: `scrape.js` — launch browser, load cookies, navigate

**Files:**
- Create: `providers/perplexity/scrape.js`
- Create: `providers/perplexity/scrape.test.js`

- [ ] **Step 1: Write smoke test (gated by `SMOKE=1`)**

Create `providers/perplexity/scrape.test.js`:

```js
import { describe, it, expect } from 'vitest';
import { launchAndNavigate } from './scrape.js';

const smoke = process.env.SMOKE === '1';

describe.skipIf(!smoke)('scrape — live browser (SMOKE=1)', () => {
  it('launches chromium, loads cookies, navigates to perplexity.ai without login wall', async () => {
    const { page, browser } = await launchAndNavigate();
    try {
      const url = page.url();
      expect(url).toMatch(/perplexity\.ai/);
      const content = await page.content();
      // Rough sanity: we landed somewhere with a prompt input visible.
      expect(content.toLowerCase()).toMatch(/perplexity/);
    } finally {
      await browser.close();
    }
  }, 60_000);
});
```

- [ ] **Step 2: Implement `launchAndNavigate`**

Create `providers/perplexity/scrape.js`:

```js
import { chromium } from 'playwright';
import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import { PerplexityAuthError, PerplexityScrapeError } from './errors.js';
import { SELECTORS } from './selectors.js';

const COOKIE_PATH = path.join(os.homedir(), '.claude', 'cookie-configs', 'perplexity.ai-cookies.json');

export async function launchAndNavigate({ threadId } = {}) {
  if (!fs.existsSync(COOKIE_PATH)) {
    throw new PerplexityAuthError();
  }
  const cookies = JSON.parse(fs.readFileSync(COOKIE_PATH, 'utf8'));

  const browser = await chromium.launch({ headless: true });
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  await context.addCookies(cookies);
  const page = await context.newPage();

  const url = threadId
    ? `https://www.perplexity.ai/search/${encodeURIComponent(threadId)}`
    : 'https://www.perplexity.ai/';

  await page.goto(url, { waitUntil: 'domcontentloaded' });

  // Fail fast if we landed on a login wall.
  if (SELECTORS.loginWallDetector) {
    const wall = await page.locator(SELECTORS.loginWallDetector).first().isVisible().catch(() => false);
    if (wall) {
      await browser.close();
      throw new PerplexityAuthError();
    }
  }

  return { browser, context, page };
}
```

- [ ] **Step 3: Run smoke test**

Run:
```bash
SMOKE=1 npm test providers/perplexity/scrape.test.js
```

Expected: 1 test PASS (browser launches, navigates, closes).

- [ ] **Step 4: Run without `SMOKE=1` — expect test to skip**

Run:
```bash
npm test providers/perplexity/scrape.test.js
```

Expected: test is skipped (`skipIf` active).

- [ ] **Step 5: Commit**

```bash
git add providers/perplexity/scrape.js providers/perplexity/scrape.test.js
git commit -m "feat(perplexity): scrape launches browser with cookies and navigates"
```

---

## Task 12: `scrape.js` — mode selection (focus is URL-based in Task 11)

**Files:**
- Modify: `providers/perplexity/scrape.js`
- Modify: `providers/perplexity/scrape.test.js`

- [ ] **Step 1: Extend smoke test**

Append to `providers/perplexity/scrape.test.js`:

```js
import { selectMode, selectFocus } from './scrape.js';

describe.skipIf(!smoke)('scrape — mode and focus selection (SMOKE=1)', () => {
  it('selects Pro mode and Academic focus without error', async () => {
    const { page, browser } = await launchAndNavigate();
    try {
      await selectMode(page, 'pro');
      await selectFocus(page, 'academic');
      // If selectors broke we'd have thrown already.
      expect(true).toBe(true);
    } finally {
      await browser.close();
    }
  }, 60_000);
});
```

- [ ] **Step 2: Implement `selectMode` (URL-based focus lives in Task 11)**

Append to `providers/perplexity/scrape.js`:

```js
const MODE_LABELS = {
  auto: 'Auto',
  pro: 'Pro',
  reasoning: 'Reasoning',
  'deep-research': 'Deep Research',
};

const FOCUS_LABELS = {
  web: 'Web',
  academic: 'Academic',
  writing: 'Writing',
};

export async function selectMode(page, mode) {
  if (mode === 'auto') return; // Auto is the default, no click needed.
  const label = MODE_LABELS[mode];
  if (!label) throw new PerplexityScrapeError('select-mode', 'unknown mode', `mode=${mode}`);

  try {
    await page.locator(SELECTORS.modeButton).first().click({ timeout: 10_000 });
    await page.locator(SELECTORS.modeOption(label)).first().click({ timeout: 10_000 });
  } catch (err) {
    throw new PerplexityScrapeError('select-mode', SELECTORS.modeButton, await page.content());
  }
}

export async function selectFocus(page, focus) {
  if (focus === 'web') return; // Web is the default.
  const label = FOCUS_LABELS[focus];
  if (!label) throw new PerplexityScrapeError('select-focus', 'unknown focus', `focus=${focus}`);

  try {
    await page.locator(SELECTORS.focusButton).first().click({ timeout: 10_000 });
    await page.locator(SELECTORS.focusOption(label)).first().click({ timeout: 10_000 });
  } catch (err) {
    throw new PerplexityScrapeError('select-focus', SELECTORS.focusButton, await page.content());
  }
}
```

- [ ] **Step 3: Run smoke test**

Run:
```bash
SMOKE=1 npm test providers/perplexity/scrape.test.js
```

Expected: 2 tests PASS.

- [ ] **Step 4: Commit**

```bash
git add providers/perplexity/scrape.js providers/perplexity/scrape.test.js
git commit -m "feat(perplexity): scrape selects mode and focus"
```

---

## Task 13: `scrape.js` — submit prompt, wait for completion, return HTML

**Files:**
- Modify: `providers/perplexity/scrape.js`
- Modify: `providers/perplexity/scrape.test.js`

- [ ] **Step 1: Extend smoke test for full fast-mode flow**

Append to `providers/perplexity/scrape.test.js`:

```js
import { scrapeOnce } from './scrape.js';

describe.skipIf(!smoke)('scrape — full fast-mode flow (SMOKE=1)', () => {
  it('returns HTML + URL for Auto/Web on a trivial prompt', async () => {
    const { html, url } = await scrapeOnce({
      prompt: 'who founded meta (formerly facebook)?',
      mode: 'auto',
      focus: 'web',
    });
    expect(typeof html).toBe('string');
    expect(html.length).toBeGreaterThan(1000);
    expect(url).toMatch(/perplexity\.ai\/search\//);
  }, 180_000);
});
```

- [ ] **Step 2: Implement `scrapeOnce`**

Append to `providers/perplexity/scrape.js`:

```js
const FAST_TIMEOUT_MS = 180_000;  // 3 min, matches spec
const NO_PROGRESS_MS = 300_000;   // 5 min, for Deep Research
const DEEP_TIMEOUT_MS = 1_800_000; // 30 min, matches spec

export async function scrapeOnce({ prompt, mode = 'auto', focus = 'web', threadId }) {
  const { browser, page } = await launchAndNavigate({ threadId });
  try {
    await selectMode(page, mode);
    await selectFocus(page, focus);

    const input = page.locator(SELECTORS.promptInput).first();
    await input.click({ timeout: 10_000 });
    await input.type(prompt, { delay: 15 });
    await page.keyboard.press(SELECTORS.submitKey);

    await waitForCompletion(page, { mode });

    const html = await page.content();
    const url = page.url();
    return { html, url };
  } finally {
    await browser.close();
  }
}

async function waitForCompletion(page, { mode }) {
  const totalTimeout = mode === 'deep-research' ? DEEP_TIMEOUT_MS : FAST_TIMEOUT_MS;
  const start = Date.now();
  let lastProgress = Date.now();

  while (Date.now() - start < totalTimeout) {
    const generating = await page.locator(SELECTORS.generatingIndicator).first().isVisible().catch(() => false);
    if (!generating) {
      // Optional done-indicator double-check
      if (SELECTORS.doneIndicator) {
        const done = await page.locator(SELECTORS.doneIndicator).first().isVisible().catch(() => false);
        if (done) return;
      } else {
        return;
      }
    }

    // For Deep Research, watch the progress text and trip "no-progress" timeout if it stalls.
    if (mode === 'deep-research' && SELECTORS.deepResearchProgressText) {
      const txt = await page.locator(SELECTORS.deepResearchProgressText).first().textContent().catch(() => null);
      if (txt) lastProgress = Date.now();
      if (Date.now() - lastProgress > NO_PROGRESS_MS) {
        throw new PerplexityTimeoutError('deep-research-no-progress', NO_PROGRESS_MS);
      }
    }

    await page.waitForTimeout(1000);
  }

  throw new PerplexityTimeoutError(mode === 'deep-research' ? 'deep-research-total' : 'fast-total', totalTimeout);
}
```

Also add `PerplexityTimeoutError` to the imports at the top:

```js
import { PerplexityAuthError, PerplexityScrapeError, PerplexityTimeoutError } from './errors.js';
```

- [ ] **Step 3: Run smoke test**

Run:
```bash
SMOKE=1 npm test providers/perplexity/scrape.test.js
```

Expected: all smoke tests PASS. Test duration ~30–60s for the full flow.

- [ ] **Step 4: Commit**

```bash
git add providers/perplexity/scrape.js providers/perplexity/scrape.test.js
git commit -m "feat(perplexity): scrapeOnce performs full fast-mode flow end-to-end"
```

---

## Task 14: `index.js` — `askPerplexity()` public API

**Files:**
- Create: `providers/perplexity/index.js`
- Create: `providers/perplexity/index.test.js`

- [ ] **Step 1: Write unit test with mocked `scrapeOnce`**

Create `providers/perplexity/index.test.js`:

```js
import { describe, it, expect, vi } from 'vitest';
import { readFileSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const fixture = (name) => readFileSync(resolve(__dirname, '__fixtures__', name), 'utf8');

vi.mock('./scrape.js', () => ({
  scrapeOnce: vi.fn(),
}));

const { scrapeOnce } = await import('./scrape.js');
const { askPerplexity } = await import('./index.js');

describe('askPerplexity', () => {
  it('returns parsed structured result', async () => {
    scrapeOnce.mockResolvedValueOnce({
      html: fixture('auto-web.html'),
      url: 'https://www.perplexity.ai/search/fixture-thread-id',
    });
    const result = await askPerplexity({ prompt: 'anything' });
    expect(result.answer.length).toBeGreaterThan(20);
    expect(result.sources.length).toBeGreaterThanOrEqual(1);
    expect(result.threadId).toBe('fixture-thread-id');
    expect(result.raw).toBeUndefined();
  });

  it('passes raw option through to parse', async () => {
    scrapeOnce.mockResolvedValueOnce({
      html: fixture('auto-web.html'),
      url: 'https://www.perplexity.ai/search/fixture-thread-id',
    });
    const result = await askPerplexity({ prompt: 'x', raw: true });
    expect(result.raw).toBeDefined();
    expect(typeof result.raw.answerHtml).toBe('string');
  });

  it('forwards mode, focus, threadId to scrapeOnce', async () => {
    scrapeOnce.mockResolvedValueOnce({
      html: fixture('pro-web.html'),
      url: 'https://www.perplexity.ai/search/continued',
    });
    await askPerplexity({ prompt: 'x', mode: 'pro', focus: 'academic', threadId: 'continued' });
    expect(scrapeOnce).toHaveBeenCalledWith(expect.objectContaining({
      prompt: 'x', mode: 'pro', focus: 'academic', threadId: 'continued',
    }));
  });
});
```

- [ ] **Step 2: Run — expect failure (no index.js yet)**

Run:
```bash
npm test providers/perplexity/index.test.js
```

Expected: FAIL — module `./index.js` not found.

- [ ] **Step 3: Implement `index.js`**

Create `providers/perplexity/index.js`:

```js
import { scrapeOnce } from './scrape.js';
import { parse } from './parse.js';

export async function askPerplexity({ prompt, mode = 'auto', focus = 'web', threadId, raw = false }) {
  if (focus === 'writing' && mode === 'deep-research') {
    throw new Error('Writing focus is incompatible with Deep Research mode');
  }
  const { html, url } = await scrapeOnce({ prompt, mode, focus, threadId });
  return parse(html, { url, mode, raw });
}
```

(Deep Research export comes in Task 17.)

- [ ] **Step 4: Run tests**

Run:
```bash
npm test providers/perplexity/index.test.js
```

Expected: 3 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add providers/perplexity/index.js providers/perplexity/index.test.js
git commit -m "feat(perplexity): askPerplexity wires scrape + parse"
```

---

## Task 15: `server.js` — `POST /ask-perplexity` endpoint

**Files:**
- Modify: `server.js`
- Create: `server.test.js` (root-level, tests both Grok and Perplexity endpoints going forward)

- [ ] **Step 1: Write supertest integration test with mocked `askPerplexity`**

Create `server.test.js` at the repo root:

```js
import { describe, it, expect, vi, beforeEach } from 'vitest';
import request from 'supertest';

vi.mock('./providers/perplexity/index.js', () => ({
  askPerplexity: vi.fn(),
  askPerplexityDeep: vi.fn(),
}));

// Also stub grok so importing server.js doesn't launch a browser during tests
vi.mock('./grok-reverse-api-grok-main.js', () => ({
  askGrok: vi.fn(),
}));

const { askPerplexity } = await import('./providers/perplexity/index.js');

// server.js listens on import; we need the app, not the listener. Refactor below exposes it.
const { app } = await import('./server.js');

describe('POST /ask-perplexity', () => {
  beforeEach(() => vi.clearAllMocks());

  it('returns structured result on success', async () => {
    askPerplexity.mockResolvedValueOnce({
      answer: 'Mark Zuckerberg founded Meta.',
      sources: [{ index: 1, title: 'Wikipedia', url: 'https://en.wikipedia.org/wiki/Meta', domain: 'en.wikipedia.org' }],
      threadId: 'abc',
    });
    const res = await request(app)
      .post('/ask-perplexity')
      .send({ prompt: 'who founded meta?' });
    expect(res.status).toBe(200);
    expect(res.body.answer).toMatch(/Zuckerberg/);
    expect(res.body.sources[0].url).toMatch(/^https?:\/\//);
    expect(res.body.threadId).toBe('abc');
  });

  it('returns 400 on missing prompt', async () => {
    const res = await request(app).post('/ask-perplexity').send({});
    expect(res.status).toBe(400);
  });

  it('returns 401 on PerplexityAuthError', async () => {
    const { PerplexityAuthError } = await import('./providers/perplexity/errors.js');
    askPerplexity.mockRejectedValueOnce(new PerplexityAuthError());
    const res = await request(app).post('/ask-perplexity').send({ prompt: 'x' });
    expect(res.status).toBe(401);
    expect(res.body.error).toBe('PerplexityAuthError');
  });
});
```

- [ ] **Step 2: Run — expect failure (server.js doesn't export app yet, endpoint doesn't exist)**

Run:
```bash
npm test server.test.js
```

Expected: FAIL — either `app` is undefined on import or the endpoint returns 404.

- [ ] **Step 3: Refactor `server.js` to export `app` and add endpoint**

Replace `server.js` with:

```js
import express from 'express';
import { askGrok } from './grok-reverse-api-grok-main.js';
import { askPerplexity } from './providers/perplexity/index.js';
import {
  PerplexityAuthError,
  PerplexityScrapeError,
  PerplexityTimeoutError,
  PerplexityParseError,
} from './providers/perplexity/errors.js';

export const app = express();
app.use(express.json());

const MAX_PORT_TRIES = 10;

app.post('/ask-grok', async (req, res) => {
  const userPrompt = req.body.prompt;
  if (!userPrompt) return res.status(400).json({ error: 'No prompt provided' });

  try {
    const grokResponse = await askGrok(userPrompt);
    res.json({ result: grokResponse });
  } catch (error) {
    res.status(500).json({
      error: 'Grok request failed',
      details: error.message,
    });
  }
});

app.post('/ask-perplexity', async (req, res) => {
  const { prompt, mode, focus, threadId, raw } = req.body || {};
  if (!prompt) return res.status(400).json({ error: 'No prompt provided' });

  try {
    const result = await askPerplexity({ prompt, mode, focus, threadId, raw });
    res.json(result);
  } catch (err) {
    return respondPerplexityError(res, err);
  }
});

function respondPerplexityError(res, err) {
  if (err instanceof PerplexityAuthError) {
    return res.status(401).json({ error: 'PerplexityAuthError', message: err.message });
  }
  if (err instanceof PerplexityTimeoutError) {
    return res.status(504).json({ error: 'PerplexityTimeoutError', stage: err.stage, timeoutMs: err.timeoutMs });
  }
  if (err instanceof PerplexityScrapeError) {
    return res.status(502).json({
      error: 'PerplexityScrapeError',
      stage: err.stage,
      selector: err.selector,
      html: err.htmlTruncated,
    });
  }
  if (err instanceof PerplexityParseError) {
    return res.status(502).json({ error: 'PerplexityParseError', reason: err.reason });
  }
  return res.status(500).json({ error: 'InternalError', message: err.message });
}

const START_PORT = Number(process.env.PORT) || 3005;

function listenWithRetry(port, attempt = 1) {
  const server = app.listen(port, () => {
    console.log(`🍺 The Tavern is open! Reverse API running on http://localhost:${port}`);
  });
  server.on('error', (error) => {
    if (error.code === 'EADDRINUSE') {
      if (attempt >= MAX_PORT_TRIES) {
        console.error(`Server failed to start: no open port found after ${MAX_PORT_TRIES} attempts starting at ${START_PORT}.`);
        process.exit(1);
      }
      const nextPort = port + 1;
      console.warn(`Port ${port} is busy, trying ${nextPort}...`);
      listenWithRetry(nextPort, attempt + 1);
      return;
    }
    console.error('Server failed to start:', error.message);
    process.exit(1);
  });
}

// Only listen when executed directly, not when imported by tests.
const isMainModule = process.argv[1] && new URL(`file://${process.argv[1]}`).href === import.meta.url;
if (isMainModule) {
  listenWithRetry(START_PORT);
}
```

`respondPerplexityError` will be reused by the Deep Research endpoints in Task 18.

- [ ] **Step 4: Run tests**

Run:
```bash
npm test server.test.js
```

Expected: 3 tests PASS.

- [ ] **Step 5: Start the server manually to verify it still boots**

Run:
```bash
npm start &
sleep 1
curl -s -X POST http://localhost:3005/ask-perplexity -H 'Content-Type: application/json' -d '{}'
kill %1
```

Expected: `{"error":"No prompt provided"}` (shows the endpoint is wired).

- [ ] **Step 6: Commit**

```bash
git add server.js server.test.js
git commit -m "feat(perplexity): add POST /ask-perplexity endpoint"
```

---

## Task 16: `jobs.js` — in-memory Deep Research job store

**Files:**
- Create: `providers/perplexity/jobs.js`
- Create: `providers/perplexity/jobs.test.js`

- [ ] **Step 1: Write failing tests**

Create `providers/perplexity/jobs.test.js`:

```js
import { describe, it, expect, beforeEach, vi, afterEach } from 'vitest';
import { createJobStore } from './jobs.js';

describe('jobs — in-memory store', () => {
  let store;

  beforeEach(() => {
    vi.useFakeTimers();
    store = createJobStore({ ttlMs: 24 * 60 * 60 * 1000 });
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('create returns a job with id and queued status', () => {
    const job = store.create();
    expect(typeof job.jobId).toBe('string');
    expect(job.jobId.length).toBeGreaterThan(10);
    expect(store.get(job.jobId)).toMatchObject({ status: 'queued' });
  });

  it('updateProgress transitions queued → running with progress text', () => {
    const { jobId } = store.create();
    store.updateProgress(jobId, 'Searching sources');
    expect(store.get(jobId)).toMatchObject({ status: 'running', progress: 'Searching sources' });
  });

  it('complete stores the result and sets status done', () => {
    const { jobId } = store.create();
    store.complete(jobId, { answer: 'a', sources: [], threadId: 't' });
    expect(store.get(jobId)).toMatchObject({ status: 'done', result: { answer: 'a', sources: [], threadId: 't' } });
  });

  it('fail stores the error and sets status failed', () => {
    const { jobId } = store.create();
    store.fail(jobId, new Error('boom'));
    expect(store.get(jobId)).toMatchObject({ status: 'failed', error: 'boom' });
  });

  it('get returns undefined for unknown id', () => {
    expect(store.get('no-such-id')).toBeUndefined();
  });

  it('completed jobs expire after TTL', () => {
    const { jobId } = store.create();
    store.complete(jobId, { answer: 'x' });
    vi.advanceTimersByTime(24 * 60 * 60 * 1000 + 1);
    expect(store.get(jobId)).toBeUndefined();
  });

  it('running jobs do NOT expire (only completed/failed do)', () => {
    const { jobId } = store.create();
    store.updateProgress(jobId, 'working');
    vi.advanceTimersByTime(24 * 60 * 60 * 1000 + 1);
    expect(store.get(jobId)).toMatchObject({ status: 'running' });
  });
});
```

- [ ] **Step 2: Run — expect failures**

Run:
```bash
npm test providers/perplexity/jobs.test.js
```

Expected: FAIL — module `./jobs.js` does not exist.

- [ ] **Step 3: Implement `jobs.js`**

Create `providers/perplexity/jobs.js`:

```js
import { randomUUID } from 'node:crypto';

export function createJobStore({ ttlMs = 24 * 60 * 60 * 1000 } = {}) {
  const jobs = new Map();

  function scheduleGc(jobId) {
    setTimeout(() => {
      const job = jobs.get(jobId);
      if (!job) return;
      if (job.status === 'done' || job.status === 'failed') {
        jobs.delete(jobId);
      }
    }, ttlMs).unref?.();
  }

  return {
    create() {
      const jobId = randomUUID();
      jobs.set(jobId, { jobId, status: 'queued', createdAt: Date.now() });
      return { jobId };
    },
    updateProgress(jobId, progress) {
      const job = jobs.get(jobId);
      if (!job) return;
      job.status = 'running';
      job.progress = progress;
    },
    complete(jobId, result) {
      const job = jobs.get(jobId);
      if (!job) return;
      job.status = 'done';
      job.result = result;
      delete job.progress;
      scheduleGc(jobId);
    },
    fail(jobId, err) {
      const job = jobs.get(jobId);
      if (!job) return;
      job.status = 'failed';
      job.error = err?.message ?? String(err);
      scheduleGc(jobId);
    },
    get(jobId) {
      return jobs.get(jobId);
    },
  };
}
```

- [ ] **Step 4: Run — all tests pass**

Run:
```bash
npm test providers/perplexity/jobs.test.js
```

Expected: 7 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add providers/perplexity/jobs.js providers/perplexity/jobs.test.js
git commit -m "feat(perplexity): add in-memory Deep Research job store"
```

---

## Task 17: `index.js` — `askPerplexityDeep()` + background runner

**Files:**
- Modify: `providers/perplexity/index.js`
- Modify: `providers/perplexity/scrape.js` (add progress callback)
- Modify: `providers/perplexity/index.test.js`

- [ ] **Step 1: Add progress-callback support to `scrape.js`**

In `providers/perplexity/scrape.js`, update `scrapeOnce` and `waitForCompletion` signatures to accept an `onProgress` callback:

```js
export async function scrapeOnce({ prompt, mode = 'auto', focus = 'web', threadId, onProgress } = {}) {
  const { browser, page } = await launchAndNavigate({ threadId });
  try {
    await selectMode(page, mode);
    await selectFocus(page, focus);

    const input = page.locator(SELECTORS.promptInput).first();
    await input.click({ timeout: 10_000 });
    await input.type(prompt, { delay: 15 });
    await page.keyboard.press(SELECTORS.submitKey);

    await waitForCompletion(page, { mode, onProgress });

    const html = await page.content();
    const url = page.url();
    return { html, url };
  } finally {
    await browser.close();
  }
}
```

And in `waitForCompletion`, fire `onProgress` when progress text changes:

```js
async function waitForCompletion(page, { mode, onProgress }) {
  const totalTimeout = mode === 'deep-research' ? DEEP_TIMEOUT_MS : FAST_TIMEOUT_MS;
  const start = Date.now();
  let lastProgress = Date.now();
  let lastProgressText = null;

  while (Date.now() - start < totalTimeout) {
    const generating = await page.locator(SELECTORS.generatingIndicator).first().isVisible().catch(() => false);
    if (!generating) {
      if (SELECTORS.doneIndicator) {
        const done = await page.locator(SELECTORS.doneIndicator).first().isVisible().catch(() => false);
        if (done) return;
      } else {
        return;
      }
    }

    if (mode === 'deep-research' && SELECTORS.deepResearchProgressText) {
      const txt = await page.locator(SELECTORS.deepResearchProgressText).first().textContent().catch(() => null);
      if (txt && txt !== lastProgressText) {
        lastProgressText = txt;
        lastProgress = Date.now();
        onProgress?.(txt);
      }
      if (Date.now() - lastProgress > NO_PROGRESS_MS) {
        throw new PerplexityTimeoutError('deep-research-no-progress', NO_PROGRESS_MS);
      }
    }

    await page.waitForTimeout(1000);
  }

  throw new PerplexityTimeoutError(mode === 'deep-research' ? 'deep-research-total' : 'fast-total', totalTimeout);
}
```

- [ ] **Step 2: Add test for `askPerplexityDeep`**

Append to `providers/perplexity/index.test.js`:

```js
import { createJobStore } from './jobs.js';

describe('askPerplexityDeep', () => {
  it('immediately returns jobId; job transitions queued → running → done', async () => {
    const { askPerplexityDeep } = await import('./index.js');
    const store = createJobStore();
    scrapeOnce.mockImplementationOnce(async ({ onProgress }) => {
      onProgress?.('Searching 3 sources');
      return {
        html: fixture('deep-research-web.html'),
        url: 'https://www.perplexity.ai/search/deep-thread',
      };
    });

    const { jobId } = askPerplexityDeep({ prompt: 'x', store });
    expect(store.get(jobId).status).toBe('queued');

    // Let the background promise resolve
    await new Promise((r) => setTimeout(r, 0));
    await new Promise((r) => setTimeout(r, 0));

    const job = store.get(jobId);
    expect(['running', 'done']).toContain(job.status);

    // Wait for completion
    for (let i = 0; i < 50 && store.get(jobId).status !== 'done'; i++) {
      await new Promise((r) => setTimeout(r, 10));
    }
    const final = store.get(jobId);
    expect(final.status).toBe('done');
    expect(final.result.threadId).toBe('deep-thread');
  });

  it('on scrape failure the job status becomes failed', async () => {
    const { askPerplexityDeep } = await import('./index.js');
    const store = createJobStore();
    scrapeOnce.mockRejectedValueOnce(new Error('scrape boom'));

    const { jobId } = askPerplexityDeep({ prompt: 'x', store });
    for (let i = 0; i < 50 && store.get(jobId).status !== 'failed'; i++) {
      await new Promise((r) => setTimeout(r, 10));
    }
    expect(store.get(jobId)).toMatchObject({ status: 'failed', error: 'scrape boom' });
  });
});
```

- [ ] **Step 3: Run — expect failures**

Run:
```bash
npm test providers/perplexity/index.test.js
```

Expected: FAIL — `askPerplexityDeep` not exported.

- [ ] **Step 4: Implement `askPerplexityDeep` in `index.js`**

Update `providers/perplexity/index.js`:

```js
import { scrapeOnce } from './scrape.js';
import { parse } from './parse.js';

export async function askPerplexity({ prompt, mode = 'auto', focus = 'web', threadId, raw = false }) {
  if (focus === 'writing' && mode === 'deep-research') {
    throw new Error('Writing focus is incompatible with Deep Research mode');
  }
  const { html, url } = await scrapeOnce({ prompt, mode, focus, threadId });
  return parse(html, { url, mode, raw });
}

export function askPerplexityDeep({ prompt, focus = 'web', threadId, raw = false, store }) {
  if (focus === 'writing') {
    throw new Error('Writing focus is incompatible with Deep Research mode');
  }
  const { jobId } = store.create();

  // Fire and forget; errors land on the job.
  (async () => {
    try {
      const { html, url } = await scrapeOnce({
        prompt,
        mode: 'deep-research',
        focus,
        threadId,
        onProgress: (text) => store.updateProgress(jobId, text),
      });
      const result = parse(html, { url, mode: 'deep-research', raw });
      store.complete(jobId, result);
    } catch (err) {
      store.fail(jobId, err);
    }
  })();

  return { jobId };
}
```

- [ ] **Step 5: Run — tests pass**

Run:
```bash
npm test providers/perplexity/index.test.js
```

Expected: 5 tests PASS (3 from Task 14 + 2 new).

- [ ] **Step 6: Commit**

```bash
git add providers/perplexity/index.js providers/perplexity/index.test.js providers/perplexity/scrape.js
git commit -m "feat(perplexity): askPerplexityDeep runs async against job store"
```

---

## Task 18: `server.js` — Deep Research endpoints

**Files:**
- Modify: `server.js`
- Modify: `server.test.js`

- [ ] **Step 1: Extend supertest suite**

Append to `server.test.js`:

```js
import { createJobStore } from './providers/perplexity/jobs.js';

describe('POST /ask-perplexity/deep + GET /ask-perplexity/deep/:jobId', () => {
  beforeEach(() => vi.clearAllMocks());

  it('POST returns 202 with jobId', async () => {
    const { askPerplexityDeep } = await import('./providers/perplexity/index.js');
    askPerplexityDeep.mockReturnValueOnce({ jobId: 'fake-uuid-123' });

    const res = await request(app)
      .post('/ask-perplexity/deep')
      .send({ prompt: 'what are the latest advancements in fusion energy?' });

    expect(res.status).toBe(202);
    expect(res.body.jobId).toBe('fake-uuid-123');
  });

  it('POST returns 400 on missing prompt', async () => {
    const res = await request(app).post('/ask-perplexity/deep').send({});
    expect(res.status).toBe(400);
  });

  it('GET returns 404 for unknown jobId', async () => {
    const res = await request(app).get('/ask-perplexity/deep/no-such-job');
    expect(res.status).toBe(404);
    expect(res.body.error).toBe('JobNotFound');
  });
});
```

- [ ] **Step 2: Run — expect failures**

Run:
```bash
npm test server.test.js
```

Expected: new tests FAIL (endpoints don't exist).

- [ ] **Step 3: Wire Deep Research endpoints in `server.js`**

In `server.js`, add the imports at the top:

```js
import { askPerplexity, askPerplexityDeep } from './providers/perplexity/index.js';
import { createJobStore } from './providers/perplexity/jobs.js';
```

(Replace the existing single-import of `askPerplexity` with the two-name form above.)

Add near the top of the file (after `app.use(express.json());`):

```js
const perplexityJobs = createJobStore();
```

Add the two new endpoints below the existing `/ask-perplexity` handler:

```js
app.post('/ask-perplexity/deep', (req, res) => {
  const { prompt, focus, threadId, raw } = req.body || {};
  if (!prompt) return res.status(400).json({ error: 'No prompt provided' });

  try {
    const { jobId } = askPerplexityDeep({ prompt, focus, threadId, raw, store: perplexityJobs });
    res.status(202).json({ jobId });
  } catch (err) {
    return respondPerplexityError(res, err);
  }
});

app.get('/ask-perplexity/deep/:jobId', (req, res) => {
  const job = perplexityJobs.get(req.params.jobId);
  if (!job) return res.status(404).json({ error: 'JobNotFound' });
  res.json(job);
});
```

- [ ] **Step 4: Run tests**

Run:
```bash
npm test server.test.js
```

Expected: 6 tests PASS (3 original + 3 new).

- [ ] **Step 5: Run the full suite**

Run:
```bash
npm test
```

Expected: all non-smoke tests PASS. No test is skipped other than the `SMOKE=1`-gated ones.

- [ ] **Step 6: Commit**

```bash
git add server.js server.test.js
git commit -m "feat(perplexity): add Deep Research async endpoints"
```

---

## Task 19: End-to-end smoke verification against live Perplexity

**Files:**
- (no code changes; manual verification)

This task confirms the full stack works end-to-end against live Perplexity. It's the ground-truth check before declaring the Node provider done.

- [ ] **Step 1: Start the server**

Run:
```bash
npm start
```

Expected: `🍺 The Tavern is open! Reverse API running on http://localhost:3005`

Leave it running in another terminal.

- [ ] **Step 2: Hit `/ask-perplexity` with Auto/Web**

Run:
```bash
curl -s -X POST http://localhost:3005/ask-perplexity \
  -H 'Content-Type: application/json' \
  -d '{"prompt":"who founded meta (formerly facebook)?"}' | jq
```

Expected within ~30–60s: JSON with `answer`, `sources`, `threadId`. Answer mentions Zuckerberg. Sources array has ≥1 entry with `url` starting with `https://`.

- [ ] **Step 3: Follow up in the same thread**

Copy the `threadId` from Step 2. Then:

```bash
THREAD_ID="<paste from step 2>"
curl -s -X POST http://localhost:3005/ask-perplexity \
  -H 'Content-Type: application/json' \
  -d "{\"prompt\":\"what year?\",\"threadId\":\"$THREAD_ID\"}" | jq
```

Expected: answer references the founding year (2004 for Facebook), which only makes sense if the thread context was preserved.

- [ ] **Step 4: Verify mode and focus switches work**

```bash
curl -s -X POST http://localhost:3005/ask-perplexity \
  -H 'Content-Type: application/json' \
  -d '{"prompt":"attention is all you need paper key findings","mode":"reasoning","focus":"academic"}' | jq
```

Expected: answer shows reasoning artifacts, sources include arXiv-class domains.

- [ ] **Step 5: Verify `raw: true`**

```bash
curl -s -X POST http://localhost:3005/ask-perplexity \
  -H 'Content-Type: application/json' \
  -d '{"prompt":"what is 2+2?","raw":true}' | jq '.raw | keys'
```

Expected: `["answerHtml", "sourcesHtml"]`

- [ ] **Step 6: Kick off Deep Research**

```bash
curl -s -X POST http://localhost:3005/ask-perplexity/deep \
  -H 'Content-Type: application/json' \
  -d '{"prompt":"comprehensive state of fusion energy research in 2026"}' | jq
```

Expected: `{"jobId":"<uuid>"}` returned in <1s.

- [ ] **Step 7: Poll the Deep Research job**

```bash
JOB_ID="<paste jobId from step 6>"
watch -n 5 "curl -s http://localhost:3005/ask-perplexity/deep/$JOB_ID | jq '{status, progress}'"
```

Expected: status transitions `queued → running` within ~10s; `progress` field updates as Perplexity iterates; eventually `status: "done"` with `result.steps[]` populated. Total run ~5 min.

- [ ] **Step 8: Verify expired jobId returns 404**

```bash
curl -s -w '%{http_code}\n' http://localhost:3005/ask-perplexity/deep/no-such-job
```

Expected: `404` with body `{"error":"JobNotFound"}`.

- [ ] **Step 9: Verify cookie failure returns 401**

Temporarily rename cookies:

```bash
mv ~/.claude/cookie-configs/perplexity.ai-cookies.json{,.bak}
curl -s -w '\nHTTP: %{http_code}\n' -X POST http://localhost:3005/ask-perplexity \
  -H 'Content-Type: application/json' \
  -d '{"prompt":"x"}'
mv ~/.claude/cookie-configs/perplexity.ai-cookies.json{.bak,}
```

Expected: HTTP 401, body has `"error":"PerplexityAuthError"` and the refresh-cookies message.

- [ ] **Step 10: Stop the server**

In the server terminal, `Ctrl+C`.

No commit needed — this task is manual verification only.

---

## Task 20: README update + plan completion commit

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Update the root README to document the new Perplexity surface**

Open `README.md`. In the "Components" list under `./ — Node.js scraper`, extend the bullet to mention Perplexity, and add usage examples below.

Find the existing block:

```markdown
- **`./` — Node.js scraper** · Express-based reverse-API approach using
  Playwright and puppeteer-extra-stealth. Entry: `node server.js`.
```

Replace it with:

```markdown
- **`./` — Node.js scraper** · Express-based reverse-API approach using
  Playwright and puppeteer-extra-stealth. Entry: `node server.js`. Exposes
  `/ask-grok` and `/ask-perplexity` (plus `/ask-perplexity/deep` for Deep
  Research async jobs). See `providers/perplexity/` for the structured
  provider module.
```

Add a new section above "Shared conventions":

```markdown
## Perplexity usage

```bash
# Fast modes (≤3 min)
curl -X POST http://localhost:3005/ask-perplexity \
  -H 'Content-Type: application/json' \
  -d '{"prompt":"your question","mode":"pro","focus":"academic"}'

# Deep Research (async, ≤30 min)
JOB=$(curl -sX POST http://localhost:3005/ask-perplexity/deep \
  -H 'Content-Type: application/json' \
  -d '{"prompt":"your research question"}' | jq -r .jobId)

curl -s http://localhost:3005/ask-perplexity/deep/$JOB | jq
```

Cookies expected at `~/.claude/cookie-configs/perplexity.ai-cookies.json`
(export via the `cookie-master-key/` Chrome extension).
```

- [ ] **Step 2: Verify the full test suite still passes**

Run:
```bash
npm test
```

Expected: all tests PASS. No regression from the README edit.

- [ ] **Step 3: Commit README**

```bash
git add README.md
git commit -m "docs: document Perplexity endpoints in root README"
```

- [ ] **Step 4: Final verification — spec checklist walkthrough**

Walk through the spec's verification checklist
(`docs/superpowers/specs/2026-04-23-perplexity-scraper-design.md`, "Verification
checklist" section). Every box except the last two (Rust CLI items) should now
be checkable. The Rust CLI items are deferred to the sibling plan.

If every applicable item checks out, the Node provider is done.
