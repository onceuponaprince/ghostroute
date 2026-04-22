# ghostroute monorepo setup — implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Initialize git in the existing `~/code/scraper/` directory as a flat monorepo called `ghostroute`, absorb the three sub-projects, rewrite READMEs, and push to a private GitHub repo.

**Architecture:** Flat monorepo with a single `.git` at the root. Three sub-projects (`./` Node scraper, `ask-grok-cli/` Rust CLI, `cookie-master-key/` Chrome extension) stay at the top level. No workspace tooling yet. Four logical initial commits. See spec for full rationale: `docs/superpowers/specs/2026-04-22-ghostroute-monorepo-setup-design.md`.

**Tech Stack:** `git`, `gh` (GitHub CLI), shell. No compilation or test execution required — this is an infrastructure migration, not code work.

**Operational note:** Many steps run destructive one-way operations (folder rename, `.git/` removal). The plan front-loads verification checks before every destructive step. If any check output differs from "Expected," stop and surface the mismatch — do not proceed.

---

## File operations summary

| Path | Operation |
| --- | --- |
| `~/code/scraper/ask-grok-cli/.git/` | delete |
| `~/code/scraper/` → `~/code/ghostroute/` | rename |
| `~/code/ghostroute/README.md` | move to `ask-grok-cli/README.md` (content unchanged) |
| `~/code/ghostroute/README.md` | create (new umbrella README) |
| `~/code/ghostroute/cookie-master-key/README.md` | create (new stub) |
| `~/code/ghostroute/.gitignore` | rewrite |
| `~/code/ghostroute/.git/` | create (via `git init`) |
| GitHub repo `ghostroute` (private) | create + push |

---

## Task 1: Pre-flight — scan for hardcoded `~/code/scraper` references

**Files:**
- Read: `~/.zshrc`, `~/.bashrc`, `~/.config/` shell configs
- Read: `~/code/scraper/.claude/settings.local.json`
- Read: `~/code/scraper/ask-grok-cli/.claude/settings.local.json`

Before renaming the folder, find anything that hardcodes the old path so it doesn't silently break.

- [ ] **Step 1: Grep shell rc files**

Run:
```bash
grep -n "code/scraper" ~/.zshrc ~/.bashrc ~/.profile ~/.zprofile 2>/dev/null
```

Expected: no output (clean), OR a list of lines — in which case note each file and line number for Step 4 of Task 3.

- [ ] **Step 2: Check `.claude/` settings files inside the repo**

Run:
```bash
grep -rn "code/scraper\|/scraper/" ~/code/scraper/.claude/ ~/code/scraper/ask-grok-cli/.claude/ 2>/dev/null
```

Expected: no output, OR file:line hits to update in Task 3.

- [ ] **Step 3: Check scripts inside the repo for self-referential paths**

Run:
```bash
grep -rn "code/scraper" ~/code/scraper --include="*.js" --include="*.rs" --include="*.json" --include="*.toml" 2>/dev/null | grep -v node_modules | grep -v target
```

Expected: no output, OR file:line hits to update in Task 3.

- [ ] **Step 4: Record findings**

If any of Steps 1-3 found matches, write them down. These will be updated in Task 3 Step 4. If all clean, proceed directly.

No commit — this task is read-only reconnaissance.

---

## Task 2: Remove `ask-grok-cli`'s local-only `.git`

**Files:**
- Delete: `~/code/scraper/ask-grok-cli/.git/`

- [ ] **Step 1: Verify `ask-grok-cli`'s git has no remotes (safety check)**

Run:
```bash
cd ~/code/scraper/ask-grok-cli && git remote -v
```

Expected: empty output. If ANY remote is listed, STOP — the spec assumed local-only. Do not delete; surface the mismatch.

- [ ] **Step 2: Record the current HEAD SHA (for lossless rollback if needed)**

Run:
```bash
cd ~/code/scraper/ask-grok-cli && git rev-parse HEAD 2>/dev/null && git log --oneline | head -20
```

Expected: a SHA and a list of commits. Copy the output somewhere (a scratch file, chat, whatever). If Step 1 confirmed no remotes, this history is about to be deleted — this is your only snapshot.

- [ ] **Step 3: Delete the nested `.git/`**

Run:
```bash
rm -rf ~/code/scraper/ask-grok-cli/.git
```

- [ ] **Step 4: Verify deletion**

Run:
```bash
ls -la ~/code/scraper/ask-grok-cli/.git 2>&1
```

Expected: `No such file or directory`.

No commit — there's no repo yet.

---

## Task 3: Rename folder `scraper/` → `ghostroute/`

**Files:**
- Rename: `~/code/scraper/` → `~/code/ghostroute/`

- [ ] **Step 1: Confirm no process holds the folder open**

Run:
```bash
lsof +D ~/code/scraper 2>/dev/null | head
```

Expected: empty output. If output shows a running node/cargo/editor process with the scraper dir open, close it first.

- [ ] **Step 2: Confirm target path doesn't exist**

Run:
```bash
ls -d ~/code/ghostroute 2>&1
```

Expected: `No such file or directory`. If it exists, STOP and surface the conflict.

- [ ] **Step 3: Rename**

Run:
```bash
mv ~/code/scraper ~/code/ghostroute
```

- [ ] **Step 4: Update path references found in Task 1**

For each `file:line` recorded in Task 1 Step 4, edit to replace `code/scraper` with `code/ghostroute`. If Task 1 found no references, skip this step.

- [ ] **Step 5: Verify move**

Run:
```bash
ls -d ~/code/ghostroute && ls -d ~/code/scraper 2>&1
```

Expected: first `ls` succeeds, second says `No such file or directory`.

No commit — still no git.

---

## Task 4: Move the old root README into `ask-grok-cli/`

**Files:**
- Move: `~/code/ghostroute/README.md` → `~/code/ghostroute/ask-grok-cli/README.md`

The existing root README describes the `ask-grok-cli` Rust CLI, not the umbrella. It moves unchanged.

- [ ] **Step 1: Confirm `ask-grok-cli/README.md` does not already exist**

Run:
```bash
ls ~/code/ghostroute/ask-grok-cli/README.md 2>&1
```

Expected: `No such file or directory`. If it exists, STOP — the plan assumes it doesn't.

- [ ] **Step 2: Move the README**

Run:
```bash
mv ~/code/ghostroute/README.md ~/code/ghostroute/ask-grok-cli/README.md
```

- [ ] **Step 3: Verify**

Run:
```bash
head -3 ~/code/ghostroute/ask-grok-cli/README.md
```

Expected: starts with `# ask-grok-cli: The Stealth Mecha-Suit`.

No commit yet — writing the new root README next.

---

## Task 5: Write the new umbrella `README.md` at the root

**Files:**
- Create: `~/code/ghostroute/README.md`

- [ ] **Step 1: Write the new root README**

Create `~/code/ghostroute/README.md` with exactly this content:

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

- Cookies live outside the repo in `~/.claude/cookie-configs/` (never committed).

See each sub-project's README for setup specifics.
```

- [ ] **Step 2: Verify**

Run:
```bash
head -3 ~/code/ghostroute/README.md
```

Expected: starts with `# ghostroute`.

---

## Task 6: Write the `cookie-master-key/README.md` stub

**Files:**
- Create: `~/code/ghostroute/cookie-master-key/README.md`

- [ ] **Step 1: Write the stub**

Create `~/code/ghostroute/cookie-master-key/README.md` with exactly this content:

```markdown
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
```

- [ ] **Step 2: Verify**

Run:
```bash
head -3 ~/code/ghostroute/cookie-master-key/README.md
```

Expected: starts with `# cookie-master-key`.

---

## Task 7: Rewrite the root `.gitignore`

**Files:**
- Rewrite: `~/code/ghostroute/.gitignore`

- [ ] **Step 1: Replace `.gitignore` with the audited version**

Overwrite `~/code/ghostroute/.gitignore` with exactly this content:

```gitignore
# Session cookies — never commit
*-cookies.json

# Node
node_modules/
dist/

# Editors / OS
.vscode/
.idea/
.DS_Store
*.log
*.bak
*.tmp
*.swp

# Environment files
*.env

# Claude Code per-machine state (matched at any depth)
**/.claude/settings.local.json
**/.claude/.swarm-memory.json

# Rust build output (defense-in-depth; ask-grok-cli/.gitignore also scopes this)
ask-grok-cli/target/
```

Notes on changes vs. the old `.gitignore` (explained in the spec):
- Removed `grok.com-cookies.json` and `x.com-cookies.json` lines (redundant with `*-cookies.json`).
- Removed `package-lock.json` (now tracked for reproducibility).
- Removed the bug line that said `.gitignore` (would have caused git to ignore this file itself).
- Added `**/.claude/...` patterns using `**/` so they match nested occurrences (e.g., `ask-grok-cli/.claude/.swarm-memory.json`). The spec's bare-slash form would have only matched at the root.

- [ ] **Step 2: Verify the file is valid**

Run:
```bash
cat ~/code/ghostroute/.gitignore
```

Expected: matches the content above, no extra blank lines or stray characters.

---

## Task 8: `git init` and commit #1 — Node scraper

**Files:**
- Create: `~/code/ghostroute/.git/`

- [ ] **Step 1: Initialize the repo with `main` as the default branch**

Run:
```bash
cd ~/code/ghostroute && git init -b main
```

Expected: `Initialized empty Git repository in /home/onceuponaprince/code/ghostroute/.git/`.

- [ ] **Step 2: Sanity-check git user config**

Run:
```bash
git config user.name && git config user.email
```

Expected: both return non-empty values. If either is empty, set them:
```bash
git config user.name "<your name>"
git config user.email "madeitdao@gmail.com"
```

- [ ] **Step 3: Confirm `.gitignore` works before staging (dry-run)**

Run:
```bash
cd ~/code/ghostroute && git status --ignored | head -40
```

Expected:
- Ignored section includes `node_modules/`, `*-cookies.json` files, any `.claude/settings.local.json`, `ask-grok-cli/target/`.
- Untracked section does NOT include any of the above.

If anything that should be ignored shows as untracked, stop and fix the `.gitignore` before proceeding.

- [ ] **Step 4: Stage commit #1 — only the Node scraper surface**

Run:
```bash
cd ~/code/ghostroute && git add \
  .gitignore \
  README.md \
  package.json \
  package-lock.json \
  server.js \
  composable-scraper.js \
  grok-reverse-api.js \
  grok-reverse-api-grok-main.js \
  search-scraper.js
```

- [ ] **Step 5: Verify staging matches expectation**

Run:
```bash
cd ~/code/ghostroute && git status
```

Expected: "Changes to be committed" lists exactly the 9 files staged above. No `ask-grok-cli/` or `cookie-master-key/` files in "Changes to be committed." Those should appear in "Untracked files."

- [ ] **Step 6: Commit**

Run:
```bash
cd ~/code/ghostroute && git commit -m "chore: initial node scraper"
```

Expected: commit created. `git log --oneline` shows exactly one commit.

---

## Task 9: Commit #2 — `ask-grok-cli`

- [ ] **Step 1: Stage the Rust sub-project**

Run:
```bash
cd ~/code/ghostroute && git add ask-grok-cli/
```

- [ ] **Step 2: Verify staging**

Run:
```bash
cd ~/code/ghostroute && git status
```

Expected: "Changes to be committed" contains `ask-grok-cli/Cargo.toml`, `ask-grok-cli/Cargo.lock`, `ask-grok-cli/.gitignore`, `ask-grok-cli/README.md`, and `ask-grok-cli/src/...` files. Does NOT contain `ask-grok-cli/target/` (ignored) or `ask-grok-cli/.claude/.swarm-memory.json` (ignored). Does NOT contain `cookie-master-key/` (untracked, saved for next commit).

- [ ] **Step 3: Commit**

Run:
```bash
cd ~/code/ghostroute && git commit -m "feat: add ask-grok-cli Rust CLI"
```

Expected: `git log --oneline` shows two commits.

---

## Task 10: Commit #3 — `cookie-master-key`

- [ ] **Step 1: Stage the Chrome extension**

Run:
```bash
cd ~/code/ghostroute && git add cookie-master-key/
```

- [ ] **Step 2: Verify staging**

Run:
```bash
cd ~/code/ghostroute && git status
```

Expected: "Changes to be committed" contains `cookie-master-key/README.md`, `cookie-master-key/manifest.json`, `cookie-master-key/popup.html`, `cookie-master-key/popup.js`. No other untracked files besides `docs/` which is held for commit #4.

- [ ] **Step 3: Commit**

Run:
```bash
cd ~/code/ghostroute && git commit -m "feat: add cookie-master-key browser extension"
```

Expected: `git log --oneline` shows three commits.

---

## Task 11: Commit #4 — `docs/` (spec and this plan)

- [ ] **Step 1: Stage the docs directory**

Run:
```bash
cd ~/code/ghostroute && git add docs/
```

- [ ] **Step 2: Verify staging**

Run:
```bash
cd ~/code/ghostroute && git status
```

Expected: "Changes to be committed" contains exactly:
- `docs/superpowers/specs/2026-04-22-ghostroute-monorepo-setup-design.md`
- `docs/superpowers/plans/2026-04-22-ghostroute-monorepo-setup.md`

Working tree is otherwise clean (no untracked files).

- [ ] **Step 3: Commit**

Run:
```bash
cd ~/code/ghostroute && git commit -m "docs: add ghostroute monorepo setup spec and plan"
```

Expected: `git log --oneline` shows four commits in order (most recent first):
1. `docs: add ghostroute monorepo setup spec and plan`
2. `feat: add cookie-master-key browser extension`
3. `feat: add ask-grok-cli Rust CLI`
4. `chore: initial node scraper`

- [ ] **Step 4: Confirm working tree is fully clean**

Run:
```bash
cd ~/code/ghostroute && git status
```

Expected: `nothing to commit, working tree clean`.

---

## Task 12: Create private GitHub repo and push

**Files:** none (remote-only).

- [ ] **Step 1: Verify `gh` is authenticated**

Run:
```bash
gh auth status
```

Expected: `Logged in to github.com as <username>`. If not logged in, run `gh auth login` interactively first and resume the plan.

- [ ] **Step 2: Check the repo name `ghostroute` is available on the personal account**

Run:
```bash
gh repo view <your-github-username>/ghostroute 2>&1 | head -3
```

Expected: `Could not resolve to a Repository with the name ...`. If the repo already exists, STOP — pick a different name or delete the existing empty repo first.

- [ ] **Step 3: Create private repo, add remote, and push in one step**

Run:
```bash
cd ~/code/ghostroute && gh repo create ghostroute --private --source=. --remote=origin --push
```

Expected:
- Output ends with `✓ Created repository <username>/ghostroute on GitHub`
- Followed by push progress and `✓ Pushed commits to https://github.com/<username>/ghostroute.git`

- [ ] **Step 4: Verify remote is set and branch tracks origin**

Run:
```bash
cd ~/code/ghostroute && git remote -v && git branch -vv
```

Expected:
- `git remote -v` shows `origin  https://github.com/<username>/ghostroute.git (fetch)` and `(push)`.
- `git branch -vv` shows `* main ... [origin/main] docs: add ghostroute monorepo setup spec and plan`.

---

## Task 13: Run the spec's verification checklist

Walk through each item from the spec (`docs/superpowers/specs/2026-04-22-ghostroute-monorepo-setup-design.md` §"Verification checklist") and confirm. Each sub-step below maps to one spec checkbox.

- [ ] **Step 1: Four commits, correct order**

Run:
```bash
cd ~/code/ghostroute && git log --oneline
```

Expected (order newest → oldest):
```
<sha> docs: add ghostroute monorepo setup spec and plan
<sha> feat: add cookie-master-key browser extension
<sha> feat: add ask-grok-cli Rust CLI
<sha> chore: initial node scraper
```

- [ ] **Step 2: Working tree clean**

Run:
```bash
cd ~/code/ghostroute && git status
```

Expected: `nothing to commit, working tree clean`.

- [ ] **Step 3: `package-lock.json` is tracked**

Run:
```bash
cd ~/code/ghostroute && git ls-files | grep package-lock.json
```

Expected: `package-lock.json`.

- [ ] **Step 4: `ask-grok-cli/.claude/.swarm-memory.json` is NOT tracked**

Run:
```bash
cd ~/code/ghostroute && git ls-files | grep swarm-memory
```

Expected: empty output.

- [ ] **Step 5: No nested `.git/` inside `ask-grok-cli/`**

Run:
```bash
ls -d ~/code/ghostroute/ask-grok-cli/.git 2>&1
```

Expected: `No such file or directory`.

- [ ] **Step 6: Root README describes ghostroute (umbrella)**

Run:
```bash
head -3 ~/code/ghostroute/README.md
```

Expected: starts with `# ghostroute`.

- [ ] **Step 7: `ask-grok-cli/README.md` has the old ask-grok-cli content**

Run:
```bash
head -3 ~/code/ghostroute/ask-grok-cli/README.md
```

Expected: starts with `# ask-grok-cli: The Stealth Mecha-Suit`.

- [ ] **Step 8: Remote is set and push succeeded**

Run:
```bash
cd ~/code/ghostroute && git remote -v
```

Expected: `origin` URL pointing at the private GitHub repo.

- [ ] **Step 9: Node scraper still runs**

Run:
```bash
cd ~/code/ghostroute && node -c server.js
```

Expected: no output (syntax-check passes). If you want a fuller check, run `node server.js` and confirm it starts listening — stop it with Ctrl-C once confirmed.

- [ ] **Step 10: Rust CLI still builds**

Run:
```bash
cd ~/code/ghostroute/ask-grok-cli && cargo check
```

Expected: `Finished ...` with no errors. (Use `cargo check` not `cargo build` — faster; we're verifying paths/deps, not re-linking.)

---

## Done

At this point the migration is complete:
- `~/code/ghostroute/` is a git repo with four clean commits.
- Remote `origin` points at a private GitHub repo of the same name.
- All three sub-projects are present, their READMEs describe the correct thing, and per-machine state files are ignored.
- Both the Node scraper and the Rust CLI still work from their new location.

The project is ready for new work (deepening existing sub-projects or adding new siblings per your option-C expansion plan).
