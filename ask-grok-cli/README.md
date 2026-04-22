# ask-grok-cli: The Stealth Mecha-Suit

`ask-grok-cli` is a native Rust CLI tool that bridges your terminal with X.com Grok. Built on `chromiumoxide`, it uses the Chrome DevTools Protocol (CDP) to automate a browser session, bypass basic bot detection, manage cross-session memory, and support Claude Code orchestration.

## Core Mechanics

- **Drunk-Typist Protocol:** Types prompts asynchronously with randomized human-like delays and an occasional typo-correct sequence.
- **MCP Walkie-Talkie:** Grok can request file content via strict JSON (`read_file`) so Claude can fetch context and continue the task.
- **Stateful Memory Campfires:** Saves project-scoped context in `.claude/.swarm-memory.json` at your Git root and reuses recent interactions.
- **Mana Bar (Token Tracker):** Uses `tiktoken-rs` (`cl100k_base`) to estimate input/output token usage.
- **Global Inventory System:** Reads authentication cookies from a global OS directory so the CLI can run from anywhere.

## Installation

Clone and install:

```bash
git clone <your-repo-url>
cd ask-grok-cli
cargo install --path .
```

Ensure `~/.cargo/bin` is in your shell `PATH` (for example in `~/.zshrc` or `~/.bashrc`).

## Configuration

`ask-grok-cli` expects cookie files under `~/.claude/cookie-configs`.

1. Create the directory:

```bash
mkdir -p ~/.claude/cookie-configs
```

2. Export Grok cookies from your logged-in browser.
3. Save at least one file matching `*-cookies.json` in that directory. Example:

```bash
mv grok.com-cookies.json ~/.claude/cookie-configs/grok.com-cookies.json
```

## Usage

```bash
ask-grok-cli --prompt "Write a short haiku about Rust."
```

Under the hood, the CLI:

- Boots a Chromium session via `chromiumoxide`.
- Injects cookies from `~/.claude/cookie-configs`.
- Creates/updates project memory at `<git-root>/.claude/.swarm-memory.json`.
- Types the prompt, waits for stable response text, and prints the answer.

## Claude Code Integration

To use Grok as a sub-agent orchestrated by Claude, create or edit `.claude/skills/ask_grok.md` with:

```markdown
# Tool Name: ask_grok
# Description: Triggers the global Rust CLI to query Grok AI.

To use this tool, execute:

`ask-grok-cli --prompt "{your_task_description}"`

Important context:
1. Grok does not have direct file access.
2. It may return JSON like `{"tool": "read_file", "path": "..."}`.
3. Claude should intercept that JSON, read the file, and rerun with context.
```

## Architecture Stack

- **Language:** Rust
- **Browser Automation:** `chromiumoxide` (native CDP)
- **CLI Parser:** `clap`
- **Serialization:** `serde`, `serde_json`
- **Token Counting:** `tiktoken-rs`
- **Async Runtime:** `tokio`
