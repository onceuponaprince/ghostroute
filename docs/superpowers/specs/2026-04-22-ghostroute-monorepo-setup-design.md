# ghostroute — monorepo setup design

**Date:** 2026-04-22
**Status:** approved, pre-implementation
**Scope:** one-time migration — initialize git, absorb existing sub-projects, rename folder, set up remote

---

## Context

The directory `~/code/scraper/` currently contains three developed-out-of-the-same-idea
sub-projects, no git at the root, and a root `README.md` that actually describes one
of the sub-projects instead of the umbrella:

| Sub-project | Language | Git state | Role |
| --- | --- | --- | --- |
| root `*.js` files | Node.js (Express, Playwright, puppeteer-extra-stealth) | none | HTTP reverse-API scraper |
| `ask-grok-cli/` | Rust (`chromiumoxide`) | **local-only `.git/`** | terminal CLI, Grok via browser automation |
| `cookie-master-key/` | JS (Chrome extension) | none | cookie-export helper for both scrapers |

The user wants to:
1. Manage all three under a single repo.
2. Expand in both directions — deepen existing sub-projects *and* add new siblings
   over time (more LLM scraping targets expected).
3. Avoid premature workspace tooling; adopt it only when real duplication appears.

## Decisions

| # | Decision | Rationale |
| --- | --- | --- |
| 1 | **Flat monorepo** (single `.git` at the root, no `apps/`/`packages/` split). | Matches user's "start flat, upgrade later" preference. Non-destructive upgrade path to a workspace layout when the second duplication appears. |
| 2 | **Flatten `ask-grok-cli`'s git** (delete its `.git/`, absorb files). | History is local-only — no remote to lose, no collaborators. |
| 3 | **Four logical initial commits** (not a single import commit). | Tells a cleaner story in `git log` and keeps each commit to a single logical change per the project's atomic-commits rule. |
| 4 | **Project name: `ghostroute`**. Rename folder to `~/code/ghostroute/` before `git init`. | Scales with option-C expansion (more LLM targets); scans as a real project; zero git cost to rename pre-init. |
| 5 | **Remote: GitHub, private, personal account.** | Remote backup without the public-README pressure. Future-proof for open-sourcing later if desired. |

## Final repo structure

```
~/code/ghostroute/          ← single .git, renamed from scraper/
├── .git/                   (new)
├── .gitignore              (audited — see §"gitignore changes")
├── README.md               (rewritten — umbrella-level, describes ghostroute)
├── docs/
│   └── superpowers/
│       └── specs/
│           └── 2026-04-22-ghostroute-monorepo-setup-design.md  (this file)
├── package.json            (existing, Node scraper deps)
├── package-lock.json       (existing, NOW TRACKED)
├── server.js               (existing)
├── composable-scraper.js   (existing)
├── grok-reverse-api.js     (existing)
├── grok-reverse-api-grok-main.js (existing)
├── search-scraper.js       (existing)
├── ask-grok-cli/
│   ├── README.md           (moved from old root README — content unchanged)
│   ├── .gitignore          (existing, unchanged)
│   ├── Cargo.toml
│   ├── Cargo.lock
│   └── src/
└── cookie-master-key/
    ├── README.md           (new, 4–5 lines)
    ├── manifest.json
    ├── popup.html
    └── popup.js
```

No files move except the root `README.md` (becomes `ask-grok-cli/README.md`). The only
on-disk deletion before git init is `ask-grok-cli/.git/`.

## Migration sequence

Written as high-level steps; exact commands belong in the implementation plan.

1. **Pre-flight**
   - Scan the working tree and user's shell config for hardcoded references to
     `~/code/scraper` that would break after rename (aliases, PATH entries,
     `.claude/settings.local.json` contents, script paths).
   - Update or note any found references.
2. **Clean sub-project state**
   - `rm -rf ~/code/scraper/ask-grok-cli/.git`
3. **Rename folder**
   - `mv ~/code/scraper ~/code/ghostroute`
4. **Rewrite READMEs**
   - Move current root `README.md` to `ask-grok-cli/README.md` (content unchanged).
   - Write new root `README.md` (umbrella; see §"README restructure").
   - Write `cookie-master-key/README.md` (stub).
5. **Update `.gitignore`** per §"gitignore changes".
6. **Init + commit in four logical steps**
   - `git init`
   - Stage root Node scraper files + new root `README.md` + `.gitignore` →
     commit: `chore: initial node scraper`
   - Stage `ask-grok-cli/` (including moved `ask-grok-cli/README.md`) →
     commit: `feat: add ask-grok-cli Rust CLI`
   - Stage `cookie-master-key/` (including new `cookie-master-key/README.md`) →
     commit: `feat: add cookie-master-key browser extension`
   - Stage `docs/` → commit: `docs: add ghostroute monorepo setup spec`
7. **Create private GitHub repo** named `ghostroute` under personal account.
8. **Add remote + push**
   - `git remote add origin <url>`
   - `git branch -M main`
   - `git push -u origin main`

## `.gitignore` changes

Current root `.gitignore` has three issues to fix and several gaps to fill.

**Remove** (all are errors or redundancies):

- `grok.com-cookies.json` — redundant, caught by `*-cookies.json` above it.
- `x.com-cookies.json` — same.
- `package-lock.json` — contradicts the global rule to pin exact versions for
  reproducibility. Lockfile should be tracked.
- `.gitignore` — tells git to ignore itself; almost certainly unintentional.

**Add:**

- `.claude/settings.local.json` — per-machine Claude Code settings.
- `.claude/.swarm-memory.json` — agent memory (13 KB already exists in
  `ask-grok-cli/.claude/`).
- `ask-grok-cli/target/` — defense-in-depth. `ask-grok-cli/.gitignore` already
  handles this scope-locally; explicit at root is safer if the sub-gitignore is
  ever removed.

`ask-grok-cli/.gitignore` stays as-is (already correctly scopes `/target`,
`/config/*.json`, and `grok.com-cookies.json` to its subdir).

## README restructure

- **Old root `README.md`** → moves to `ask-grok-cli/README.md`, unchanged. The
  content is already correctly scoped to that sub-project.
- **New root `README.md`** describes the umbrella at the `ghostroute` level.
  Draft:

  ```markdown
  # ghostroute

  Tools for scraping and automating LLM web UIs, starting with X.com Grok.

  ## Components

  - **`./` — Node.js scraper** · Express-based reverse-API approach using
    Playwright and puppeteer-extra-stealth. Entry: `node server.js`.
  - **`ask-grok-cli/` — Rust CLI** · Terminal-first Grok client built on
    `chromiumoxide`. Usable standalone or orchestrated by Claude Code.
  - **`cookie-master-key/` — Chrome extension** · Exports session cookies
    from x.com / grok.com in the format the scrapers expect.

  ## Shared conventions

  - Cookies live outside the repo in `~/.claude/cookie-configs/` (never
    committed).

  See each sub-project's README for setup specifics.
  ```
- **New `cookie-master-key/README.md`** — short stub: what the extension does,
  how to load it unpacked in Chrome, format of the cookie JSON it produces.

## Out of scope for this migration

- Workspace tooling (pnpm workspaces, Cargo `[workspace]`). Deferred until the
  second duplication appears.
- Moving Node files into a subdirectory (e.g., `node-scraper/`). Deferred —
  cheaper to do together with the workspace refactor if/when it happens.
- CI/CD setup (GitHub Actions). Out of scope; add when the first automated check
  is actually needed.
- Rewriting any code. This migration changes only file locations, git state, and
  documentation.

## Verification checklist

After migration, the following should be true:

- [ ] `cd ~/code/ghostroute && git log --oneline` shows exactly four commits in
      the order: node scraper → ask-grok-cli → cookie-master-key → docs.
- [ ] `git status` is clean.
- [ ] `package-lock.json` is tracked (shows in `git ls-files`).
- [ ] `ask-grok-cli/.claude/.swarm-memory.json` is **not** tracked.
- [ ] `ask-grok-cli/` has no nested `.git/` directory.
- [ ] Root `README.md` describes `ghostroute` (umbrella), not `ask-grok-cli`.
- [ ] `ask-grok-cli/README.md` contains the original ask-grok-cli README content.
- [ ] `git remote -v` shows the private GitHub remote, and `git push` succeeded.
- [ ] Running `node server.js` and `cargo run --manifest-path ask-grok-cli/Cargo.toml`
      both still work (no path-related regressions from the folder rename).
