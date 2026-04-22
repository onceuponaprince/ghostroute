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
use automation::typing::{human_pause, human_type_with_typos};
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

fn resolve_project_root() -> PathBuf {
    let git_root_output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output();

    if let Ok(output) = git_root_output {
        if output.status.success() {
            return PathBuf::from(String::from_utf8_lossy(&output.stdout).trim().to_string());
        }
    }

    env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

fn resolve_memory_file_path() -> PathBuf {
    let project_root = resolve_project_root();
    let claude_dir = project_root.join(".claude");
    if !claude_dir.exists() {
        fs::create_dir_all(&claude_dir)
            .expect("Failed to build the .claude Campfire directory!");
    }

    claude_dir.join(".swarm-memory.json")
}

fn resolve_cookie_configs_dir() -> PathBuf {
    let home_dir = env::var("HOME").expect("Could not find OS Home Directory!");
    PathBuf::from(home_dir)
        .join(".claude")
        .join("cookie-configs")
}

fn resolve_global_cookie_path() -> Result<PathBuf> {
    let cookie_configs_dir = resolve_cookie_configs_dir();
    if !cookie_configs_dir.exists() {
        fs::create_dir_all(&cookie_configs_dir).with_context(|| {
            format!(
                "Failed to create cookie directory at {}",
                cookie_configs_dir.display()
            )
        })?;
    }

    find_cookie_file_in_dir(&cookie_configs_dir).with_context(|| {
        format!(
            "No cookie file found. Place a '*-cookies.json' file in {}",
            cookie_configs_dir.display()
        )
    })
}

fn find_cookie_file_in_dir(dir: &Path) -> Result<PathBuf> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file()
            && path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.ends_with("-cookies.json"))
        {
            return Ok(path);
        }
    }

    anyhow::bail!("No '*-cookies.json' file found in {}", dir.display());
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
    let started_at = Instant::now();

    ui::print_banner();

    eprintln!("{} Calibrating GPS coordinates...", "[System]".blue());

    let global_cookie_path = resolve_global_cookie_path()?;
    let cookie_data = fs::read_to_string(&global_cookie_path).with_context(|| {
        format!(
            "Failed to read cookie file {}. Place a '*-cookies.json' file in {}",
            global_cookie_path.display(),
            resolve_cookie_configs_dir().display()
        )
    })?;

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
let compiled_prompt = build_compiled_prompt(&save_state, &args.prompt);

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
    // Instantly paste the previous context. Normalise newlines because CDP's
    // single-char keymap has no entry for '\n' and chromiumoxide's type_str
    // falls back to a per-char press which errors on keys it can't find.
    let context_only = compiled_prompt
        .replace(&args.prompt, "")
        .replace(['\n', '\r', '\t'], " ");

    chat_input
        .type_str(&context_only)
        .await
        .context("Failed to paste previous context into input field")?;
}
        human_pause(250, 700).await;

        human_type_with_typos(&chat_input, &args.prompt)
            .await
            .context("Failed while typing prompt into input field")?;
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
            &args.prompt,
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
            text: args.prompt.clone(),
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
        build_compiled_prompt, find_cookie_file_in_dir, resolve_cookie_configs_dir,
        resolve_memory_file_path, Dialogue, SaveState,
    };
    use std::fs;
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

        let found = find_cookie_file_in_dir(&temp_dir).expect("cookie file should be discovered");
        assert_eq!(found, cookie_file);

        fs::remove_dir_all(&temp_dir).expect("temp directory cleanup should succeed");
    }

    #[test]
    fn resolves_memory_file_within_claude_directory() {
        let path = resolve_memory_file_path();
        assert_eq!(path.file_name().and_then(|n| n.to_str()), Some(".swarm-memory.json"));
        assert_eq!(path.parent().and_then(|p| p.file_name()).and_then(|n| n.to_str()), Some(".claude"));
    }
}
