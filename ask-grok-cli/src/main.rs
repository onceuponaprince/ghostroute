use anyhow::{Context, Result};
use clap::Parser;
use colored::Colorize;
use std::time::Instant;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::env;
use std::process::Command;

mod automation;
mod browser;
mod cli;
mod config;
mod ui;

use automation::locators::{find_visible_locator, Position};
use automation::response::collect_response_details;
use automation::typing::{human_pause, human_type_with_typos, paste_text};
use browser::bootstrap_browser_session;
use cli::args::Args;
use config::{INPUT_SELECTOR, INPUT_TIMEOUT_MS, RESPONSE_SELECTOR, RESPONSE_TIMEOUT_MS};

// The individual dialogue lines
#[derive(Serialize, Deserialize, Debug, Clone)]
struct Dialogue {
    speaker: String,
    text: String,
}

// The complete Save File
#[derive(Serialize, Deserialize, Debug, Default)]
struct SaveState {
    history: Vec<Dialogue>,
    total_mana_spent: usize,
}

fn resolve_project_root_if_present() -> Option<PathBuf> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if stdout.is_empty() {
        None
    } else {
        Some(PathBuf::from(stdout))
    }
}

/// Decide where memory lives without touching the filesystem. Pure so the
/// resolution logic is testable in isolation.
///
/// Use the project's `.claude/.swarm-memory.json` only when a `.claude/`
/// directory **already exists** at git root — i.e. the user has intentionally
/// opted that project into Claude-scoped state. Falls back to the global
/// `~/.claude/.swarm-memory.json` (next to cookie-configs) otherwise.
///
/// The previous behaviour silently created `.claude/` in whatever cwd the
/// binary launched from, which polluted unrelated repos with grok memory
/// when the operator was in the wrong tree at invocation time.
fn resolve_memory_file_path_with(project_root: Option<&Path>, home_dir: &Path) -> PathBuf {
    if let Some(root) = project_root {
        let project_claude = root.join(".claude");
        if project_claude.is_dir() {
            return project_claude.join(".swarm-memory.json");
        }
    }
    home_dir.join(".claude").join(".swarm-memory.json")
}

fn resolve_memory_file_path() -> PathBuf {
    let project_root = resolve_project_root_if_present();
    let home = env::var("HOME").expect("Could not find OS Home Directory!");
    let home_dir = PathBuf::from(home);
    let path = resolve_memory_file_path_with(project_root.as_deref(), &home_dir);

    // Only create the parent if it doesn't exist. Project branch already
    // confirmed `.claude/` exists; global branch may need ~/.claude/ created
    // on first run. Either way: never silently create .claude/ in a random cwd.
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)
                .expect("Failed to build the .claude Campfire directory!");
        }
    }
    path
}

fn resolve_cookie_configs_dir() -> PathBuf {
    let home_dir = env::var("HOME").expect("Could not find OS Home Directory!");
    PathBuf::from(home_dir)
        .join(".claude")
        .join("cookie-configs")
}

fn resolve_global_cookie_paths() -> Result<Vec<PathBuf>> {
    let cookie_configs_dir = resolve_cookie_configs_dir();
    if !cookie_configs_dir.exists() {
        fs::create_dir_all(&cookie_configs_dir).with_context(|| {
            format!(
                "Failed to create cookie directory at {}",
                cookie_configs_dir.display()
            )
        })?;
    }

    let paths = find_cookie_files_in_dir(&cookie_configs_dir)?;
    if paths.is_empty() {
        anyhow::bail!(
            "No '*-cookies.json' file found in {}",
            cookie_configs_dir.display()
        );
    }
    Ok(paths)
}

fn find_cookie_files_in_dir(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file()
            && path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.ends_with("-cookies.json"))
        {
            paths.push(path);
        }
    }
    paths.sort();
    Ok(paths)
}

/// Read every `*-cookies.json` in the cookie dir and merge their cookie
/// arrays into one JSON-encoded string. Grok's auth is bound to both
/// `grok.com` and `x.com` (X SSO), so the previous "first file wins"
/// loader silently dropped half the auth state.
fn load_merged_cookies(paths: &[PathBuf]) -> Result<String> {
    let mut merged: Vec<serde_json::Value> = Vec::new();
    for path in paths {
        let data = fs::read_to_string(path)
            .with_context(|| format!("Failed to read cookie file {}", path.display()))?;
        let arr: Vec<serde_json::Value> = serde_json::from_str(&data)
            .with_context(|| format!("Cookie file {} is not a JSON array", path.display()))?;
        eprintln!(
            "[Infiltrating] Loaded {} cookies from {}",
            arr.len(),
            path.file_name().unwrap_or_default().to_string_lossy()
        );
        merged.extend(arr);
    }
    Ok(serde_json::to_string(&merged)?)
}

fn build_compiled_prompt(save_state: &SaveState, user_prompt: &str) -> String {
    let mut compiled_prompt = String::new();

    // -- NEW: The MCP Walkie-Talkie Protocol --
    compiled_prompt.push_str("=== SYSTEM DIRECTIVE ===\n");
    compiled_prompt.push_str("You are Grok, an AI sub-agent. Your manager is Claude.\n");
    compiled_prompt.push_str("You DO NOT have direct access to the user's file system or codebase.\n");
    compiled_prompt.push_str("If the prompt requires you to know the contents of a specific file to answer correctly, DO NOT guess or hallucinate code.\n");
    compiled_prompt.push_str("Instead, you must ask Claude to read the file for you by outputting EXACTLY this JSON and nothing else:\n");
    compiled_prompt.push_str("{\"tool\": \"read_file\", \"path\": \"the/file/path.ext\"}\n");
    compiled_prompt.push_str("========================\n\n");

    if !save_state.history.is_empty() {
        compiled_prompt.push_str("PREVIOUS CONTEXT:\n");
        let recent_history: Vec<_> = save_state.history.iter().rev().take(4).rev().collect();
        for dialogue in recent_history {
            compiled_prompt.push_str(&format!("{}: {}\n", dialogue.speaker, dialogue.text));
        }
        compiled_prompt.push_str("\nCURRENT TASK:\n");
    }

    compiled_prompt.push_str(user_prompt);
    compiled_prompt
}


#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // ArgGroup guarantees exactly one of (prompt, prompt_file) is set.
    let prompt: String = match (&args.prompt, &args.prompt_file) {
        (Some(p), _) => p.clone(),
        (None, Some(path)) => fs::read_to_string(path).with_context(|| {
            format!("Failed to read --prompt-file {}", path.display())
        })?,
        (None, None) => unreachable!("clap ArgGroup requires one of prompt/prompt_file"),
    };

    let started_at = Instant::now();

    ui::print_banner();

    eprintln!("{} Calibrating GPS coordinates...", "[System]".blue());

    let cookie_paths = resolve_global_cookie_paths()?;
    let cookie_data = load_merged_cookies(&cookie_paths)?;

    let memory_file_path = resolve_memory_file_path();
    let memory_file = memory_file_path.to_str().unwrap();

eprintln!("{} Loading save state from: {}", "[System]".blue(), memory_file);

// 5. Load the save state if it exists, otherwise start with a fresh slate
let mut save_state = if PathBuf::from(memory_file).exists() {
    let data = fs::read_to_string(memory_file).unwrap_or_else(|_| {
        eprintln!("{} Failed to read existing save file. Starting with a fresh slate.", "[Warning]".yellow());
        String::new()
    });
    serde_json::from_str(&data).unwrap_or_else(|_| {
        eprintln!("{} Save file is corrupted or unreadable. Starting with a fresh slate.", "[Warning]".yellow());
        SaveState::default()
    })
} else {
    eprintln!("{} No existing save file found. Starting with a fresh slate.", "[System]".blue());
    SaveState::default()
};

// 2. COMPILE THE FULL PROMPT (Memory + New Task)
let compiled_prompt = build_compiled_prompt(&save_state, &prompt);

// 3. THE MANA CHECK (Token Counting)
// We use the standard cl100k_base encoding (used by GPT-4) as a highly accurate proxy for Grok
let bpe = tiktoken_rs::cl100k_base().unwrap();
let input_tokens = bpe.encode_with_special_tokens(&compiled_prompt).len();

eprintln!("{} Input Cost: {} Mana (Tokens)", "[Mana Bar]".magenta(), input_tokens);

    eprintln!("[DIAGNOSTIC] args.headless = {}", args.headless);
    eprintln!("[Equipping Mecha Suit] Launching Chromium (via chromiumoxide)...");

    eprintln!("{} Injecting cookies...", "[Infiltrating]".magenta());
        let (mut browser, page) = bootstrap_browser_session(&cookie_data, args.headless).await?;
    eprintln!(
        "{} Cookies injected successfully. We are now firmly inside the castle walls, disguised as a trusted visitor.",
        "[Infiltrating]".green().bold()
    );

    let run_result: Result<String> = async {
        // bootstrap_browser_session already navigated to grok.com and applied cookies.
        // Read state so we can detect login/challenge redirects before proceeding.
        let current_url = page.url().await.ok().flatten().unwrap_or_default();
        let title = page.get_title().await.ok().flatten().unwrap_or_default();
        eprintln!("[Nav State] url={} title=\"{}\"", current_url, title);
        eprintln!("[Timing] After navigation: {}ms", started_at.elapsed().as_millis());

        let lower_url = current_url.to_lowercase();
        if lower_url.contains("login")
            || lower_url.contains("account/access")
            || lower_url.contains("suspended")
            || lower_url.contains("challenge")
            || lower_url.contains("captcha")
        {
            anyhow::bail!(
                "Account/access block detected from URL '{}' (title='{}'). You may be logged out, rate limited, or under challenge.",
                current_url, title
            );
        }

        human_pause(500, 1300).await;

        eprintln!(
            "{} Grok homepage loaded. Scanning for input field...",
            "[Scouting Perimeter]".magenta()
        );
        eprintln!("{} Engaging Drunk-Typist protocol...", "[Engaging]".red());

        let chat_input = find_visible_locator(
            &page,
            INPUT_SELECTOR,
            INPUT_TIMEOUT_MS,
            "Input field lookup",
            Position::First,
        )
        .await?;
        eprintln!("[Timing] Input located: {}ms", started_at.elapsed().as_millis());

        chat_input
            .click()
            .await
            .context("Failed to click Grok input field")?;
        if !save_state.history.is_empty() {
            // Instant-paste the previous context via CDP `Input.insertText`,
            // which bypasses chromiumoxide's per-char keymap entirely. Newlines
            // would submit the chat, so collapse \n\r\t -> space first.
            let context_only = compiled_prompt
                .replace(&prompt, "")
                .replace(['\n', '\r', '\t'], " ");

            paste_text(&page, &context_only)
                .await
                .context("Failed to paste previous context into input field")?;
        }
        human_pause(250, 700).await;

        if args.instant_paste {
            eprintln!("[Engaging] --instant-paste set; skipping Drunk-Typist");
            paste_text(&page, &prompt)
                .await
                .context("Failed while instant-pasting prompt")?;
        } else {
            human_type_with_typos(&page, &chat_input, &prompt)
                .await
                .context("Failed while typing prompt into input field")?;
        }
        human_pause(300, 900).await;

        chat_input
            .press_key("Enter")
            .await
            .context("Failed to submit prompt with Enter key")?;

        eprintln!("[Attack Landed] Waiting for Grok's response...");

        let _response_container = find_visible_locator(
            &page,
            RESPONSE_SELECTOR,
            RESPONSE_TIMEOUT_MS,
            "Response container lookup",
            Position::Last,
        )
        .await?;
        eprintln!(
            "[Timing] Response container visible: {}ms",
            started_at.elapsed().as_millis()
        );

        let details = collect_response_details(
            &page,
            RESPONSE_SELECTOR,
            &prompt,
            RESPONSE_TIMEOUT_MS as u64,
        )
        .await?;

        let output_tokens = bpe.encode_with_special_tokens(&details.answer).len();
        let total_quest_mana = input_tokens + output_tokens;
        save_state.total_mana_spent += total_quest_mana;

        eprintln!(
            "{} Output: {} Mana | Session Total: {} Mana",
            "[Mana Bar]".magenta(),
            output_tokens,
            save_state.total_mana_spent
        );

        save_state.history.push(Dialogue {
            speaker: "User".to_string(),
            text: prompt.clone(),
        });
        save_state.history.push(Dialogue {
            speaker: "Grok".to_string(),
            text: details.answer.clone(),
        });

        let updated_json = serde_json::to_string_pretty(&save_state).unwrap();
        fs::write(memory_file, updated_json).expect("Failed to save the game state!");

        eprintln!(
            "[Response Meta] paragraphs={} chars={} preview=\"{}\"",
            details.paragraph_count, details.char_count, details.preview
        );
        eprintln!(
            "[Timing] Response collected: {}ms",
            started_at.elapsed().as_millis()
        );
        eprintln!("[Victory] Response ready.");
        Ok(details.answer)
    }
    .await;

    if let Err(close_err) = browser.close().await {
        eprintln!("[CLEANUP WARNING] Failed to close browser: {}", close_err);
    }

    match run_result {
        Ok(answer) => println!("{}", answer),
        Err(error) => eprintln!("[WASTED] The bouncer caught us: {:#}", error),
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        build_compiled_prompt, find_cookie_files_in_dir, resolve_cookie_configs_dir,
        resolve_memory_file_path_with, Dialogue, SaveState,
    };
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn build_compiled_prompt_preserves_mcp_walkie_talkie_directive() {
        let save_state = SaveState {
            history: vec![Dialogue {
                speaker: "User".to_string(),
                text: "Earlier context".to_string(),
            }],
            total_mana_spent: 0,
        };

        let compiled_prompt = build_compiled_prompt(&save_state, "Test prompt");

        assert!(compiled_prompt.contains("=== SYSTEM DIRECTIVE ==="));
        assert!(compiled_prompt.contains("Instead, you must ask Claude to read the file for you"));
        assert!(compiled_prompt.contains("PREVIOUS CONTEXT:"));
        assert!(compiled_prompt.ends_with("Test prompt"));
    }

    #[test]
    fn resolves_cookie_configs_directory_name() {
        let path = resolve_cookie_configs_dir();
        assert_eq!(
            path.file_name().and_then(|n| n.to_str()),
            Some("cookie-configs")
        );
    }

    #[test]
    fn finds_cookie_file_with_expected_suffix() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("System time should be after UNIX_EPOCH")
            .as_nanos();
        let temp_dir = std::env::temp_dir().join(format!("ask-grok-cli-cookie-test-{nonce}"));
        fs::create_dir_all(&temp_dir).expect("temp directory should be created");

        let cookie_file = temp_dir.join("grok.com-cookies.json");
        fs::write(&cookie_file, "[]").expect("cookie fixture should be written");

        let found = find_cookie_files_in_dir(&temp_dir).expect("cookie files should be discovered");
        assert_eq!(found, vec![cookie_file]);

        fs::remove_dir_all(&temp_dir).expect("temp directory cleanup should succeed");
    }

    fn unique_temp_dir(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("System time should be after UNIX_EPOCH")
            .as_nanos();
        std::env::temp_dir().join(format!("ask-grok-cli-{label}-{nonce}"))
    }

    #[test]
    fn memory_path_uses_project_claude_dir_when_present() {
        let project_root = unique_temp_dir("project");
        let home = unique_temp_dir("home");
        fs::create_dir_all(project_root.join(".claude")).expect("project .claude/ should exist");

        let path = resolve_memory_file_path_with(Some(&project_root), &home);

        assert_eq!(path, project_root.join(".claude").join(".swarm-memory.json"));
        fs::remove_dir_all(&project_root).expect("project temp cleanup");
    }

    #[test]
    fn memory_path_falls_back_to_home_when_project_claude_missing() {
        let project_root = unique_temp_dir("project-no-claude");
        fs::create_dir_all(&project_root).expect("project root should exist");
        let home = unique_temp_dir("home");

        let path = resolve_memory_file_path_with(Some(&project_root), &home);

        assert_eq!(path, home.join(".claude").join(".swarm-memory.json"));
        // Pure function must NOT have created the project .claude/ — that was
        // exactly the silent-pollution bug this fix exists to close.
        assert!(!project_root.join(".claude").exists());
        fs::remove_dir_all(&project_root).expect("project temp cleanup");
    }

    #[test]
    fn memory_path_uses_home_when_no_project_root() {
        let home = unique_temp_dir("home");
        let path = resolve_memory_file_path_with(None, &home);
        assert_eq!(path, home.join(".claude").join(".swarm-memory.json"));
    }
}
