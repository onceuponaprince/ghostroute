//! fast-travel-cli — read existing AI-chat conversations into Claude Code
//! without blowing the context budget.
//!
//! Provider-agnostic dispatcher (see `Provider`). Browser/cookies/stealth
//! live in the `ghostroute-browser` shared crate.

use chromiumoxide::{Browser, Page};
use clap::Parser;
use colored::Colorize;
use anyhow::Context;
use ghostroute_browser::{
    capture_dump_dom, cookie_file_path, default_profile_dir, detect_interstitial,
    ensure_profile_dir, human_pause, install_stealth, launch, load_cookie_data, parse_cookies,
    LaunchOpts, ProfileDir,
};
use serde::Deserialize;
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::time::sleep;

type Result<T> = anyhow::Result<T>;

#[derive(Parser, Debug)]
#[command(
    name = "fast-travel-cli",
    about = "Carry conversations from Gemini/ChatGPT/Claude/Perplexity/Grok into Claude Code without blowing the context budget"
)]
struct Args {
    /// Conversation URL. Provider is auto-detected from the URL host
    /// (e.g. https://gemini.google.com/app/<id>, https://chatgpt.com/c/<id>,
    /// https://claude.ai/chat/<id>, https://www.perplexity.ai/search/<slug>,
    /// https://grok.com/chat/<id>). Required unless --init-all is set.
    #[arg(long)]
    conversation_url: Option<String>,

    /// Show a Chromium window. Required for first-time CF challenge solves.
    #[arg(long, default_value_t = false)]
    visible: bool,

    /// Persistent profile directory. When set, cookies/storage/cache survive
    /// across runs — once a profile holds `cf_clearance`, headless runs
    /// generally bypass Cloudflare. Defaults to
    /// `~/.cache/ghostroute/profiles/<provider>` when unset.
    /// Mutually exclusive with --chrome-profile.
    #[arg(long, conflicts_with = "chrome_profile")]
    profile_dir: Option<PathBuf>,

    /// Name of an existing Chrome profile to launch against (looked up by
    /// display name in `~/.config/google-chrome/Local State`, e.g. "LLM").
    /// Inherits all cookies/extensions/saved-logins from your real Chrome
    /// profile — so the LLM logins you already have just work.
    /// Chrome must be CLOSED while this flag is in use; otherwise
    /// SingletonLock collides.
    #[arg(long)]
    chrome_profile: Option<String>,

    /// Bypass extraction; capture rich DOM diagnostics and write to stdout
    /// (or the path given by --dump-out). Skips the stub-selector guard.
    #[arg(long, default_value_t = false)]
    dump_dom: bool,

    /// Optional file to write the --dump-dom output to. Defaults to stdout.
    #[arg(long)]
    dump_out: Option<PathBuf>,

    /// Seconds to wait after navigation before capturing in --dump-dom mode.
    /// Jittered -20%..+40%.
    #[arg(long, default_value_t = 8)]
    dump_settle_secs: u64,

    /// Open one persistent-profile Chromium with tabs for every provider so
    /// you can log in / solve any Cloudflare challenges in one go. Captures
    /// resulting state into the profile dir for all subsequent runs.
    #[arg(long, default_value_t = false)]
    init_all: bool,

    /// Subset of providers for --init-all. Defaults to all five.
    #[arg(long, value_delimiter = ',')]
    init_providers: Vec<String>,

    /// Max seconds to keep the --init-all browser open. Acts as a hard
    /// safety timeout even when stdin is a tty. Default 600s (10 min) gives
    /// plenty of time to log into all five providers.
    #[arg(long, default_value_t = 600)]
    init_wait_secs: u64,

    /// Optional sentinel file. If set, --init-all polls for this file and
    /// closes the browser when it appears. Lets you signal "I'm done" from
    /// another terminal: `touch /tmp/done`. Combined with --init-wait-secs
    /// for a hard upper bound.
    #[arg(long)]
    init_done_file: Option<PathBuf>,
}

#[derive(Deserialize, Debug)]
struct Message {
    role: String,
    content: String,
}

#[derive(Deserialize, Debug)]
struct ExtractionResult {
    messages: Vec<Message>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Provider {
    Gemini,
    ChatGpt,
    Claude,
    Perplexity,
    Grok,
}

impl Provider {
    fn name(&self) -> &'static str {
        match self {
            Provider::Gemini => "gemini",
            Provider::ChatGpt => "chatgpt",
            Provider::Claude => "claude",
            Provider::Perplexity => "perplexity",
            Provider::Grok => "grok",
        }
    }

    fn bootstrap_url(&self) -> &'static str {
        match self {
            Provider::Gemini => "https://gemini.google.com",
            Provider::ChatGpt => "https://chatgpt.com",
            Provider::Claude => "https://claude.ai",
            Provider::Perplexity => "https://www.perplexity.ai",
            Provider::Grok => "https://grok.com",
        }
    }

    fn cookie_host(&self) -> &'static str {
        match self {
            Provider::Gemini => "gemini.google.com",
            Provider::ChatGpt => "chatgpt.com",
            Provider::Claude => "claude.ai",
            Provider::Perplexity => "perplexity.ai",
            Provider::Grok => "grok.com",
        }
    }

    fn auth_redirect_hosts(&self) -> &'static [&'static str] {
        match self {
            Provider::Gemini => &["accounts.google.com", "/signin", "/login"],
            Provider::ChatGpt => &["auth.openai.com", "/auth/login", "/login"],
            Provider::Claude => &["/login", "/auth"],
            Provider::Perplexity => &["/login", "/sign-in"],
            Provider::Grok => &["/login", "x.com/i/flow/login"],
        }
    }

    fn wait_script(&self) -> &'static str {
        match self {
            Provider::Gemini => WAIT_GEMINI,
            // TODO(prince): replace WAIT_STUB once the per-provider DOM dump
            // gives us a stable selector to wait on.
            Provider::ChatGpt => WAIT_STUB,
            Provider::Claude => WAIT_STUB,
            Provider::Perplexity => WAIT_STUB,
            Provider::Grok => WAIT_STUB,
        }
    }

    fn extract_script(&self) -> &'static str {
        match self {
            Provider::Gemini => EXTRACT_GEMINI,
            Provider::ChatGpt => EXTRACT_STUB,
            Provider::Claude => EXTRACT_STUB,
            Provider::Perplexity => EXTRACT_STUB,
            Provider::Grok => EXTRACT_STUB,
        }
    }

    fn all() -> &'static [Provider] {
        &[
            Provider::Claude,
            Provider::ChatGpt,
            Provider::Gemini,
            Provider::Perplexity,
            Provider::Grok,
        ]
    }

    fn from_name(name: &str) -> Result<Provider> {
        let n = name.trim().to_lowercase();
        match n.as_str() {
            "gemini" => Ok(Provider::Gemini),
            "chatgpt" | "openai" => Ok(Provider::ChatGpt),
            "claude" | "anthropic" => Ok(Provider::Claude),
            "perplexity" | "pplx" => Ok(Provider::Perplexity),
            "grok" | "x" => Ok(Provider::Grok),
            _ => anyhow::bail!("Unknown provider name: '{name}'"),
        }
    }
}

const WAIT_STUB: &str = "(() => false)();";
const EXTRACT_STUB: &str = "(() => ({ messages: [] }))();";

const WAIT_GEMINI: &str = r#"
    (() => {
        const root = document.querySelector('#chat-history, .chat-history');
        if (root && root.querySelector('message-content, user-query, model-response, .markdown.markdown-main-panel')) {
            return true;
        }
        return document.querySelector('user-query, model-response') !== null;
    })();
"#;

const EXTRACT_GEMINI: &str = r#"
    (() => {
        const UI_LABELS = new Set(['You said', 'Show thinking', 'Gemini said']);
        const stripLeadingLabels = (text) => {
            const lines = (text || '').split('\n');
            let start = 0;
            while (start < lines.length && (lines[start].trim() === '' || UI_LABELS.has(lines[start].trim()))) {
                start++;
            }
            return lines.slice(start).join('\n').trim();
        };
        const root = document.querySelector('#chat-history') ||
                     document.querySelector('.chat-history') || document;
        const messages = [];
        const messageContents = root.querySelectorAll('message-content');
        if (messageContents.length > 0) {
            messageContents.forEach(node => {
                const markdown = node.querySelector('.markdown.markdown-main-panel');
                const role = markdown ? 'model' : 'user';
                const raw = markdown ? markdown.innerText : node.innerText;
                const content = stripLeadingLabels(raw || '');
                if (content.length > 0) messages.push({ role, content });
            });
            return { messages };
        }
        root.querySelectorAll('user-query, model-response').forEach(node => {
            const tag = (node.tagName || '').toLowerCase();
            const role = tag === 'user-query' ? 'user' : 'model';
            const content = stripLeadingLabels(node.innerText || '');
            if (content.length > 0) messages.push({ role, content });
        });
        return { messages };
    })();
"#;

fn detect_provider(url: &str) -> Result<Provider> {
    const PROVIDERS: &[(Provider, &[&str])] = &[
        (Provider::Gemini, &["gemini.google.com"]),
        (Provider::ChatGpt, &["chatgpt.com", "chat.openai.com"]),
        (Provider::Claude, &["claude.ai"]),
        (Provider::Perplexity, &["perplexity.ai"]),
        (Provider::Grok, &["grok.com", "x.com/i/grok"]),
    ];
    let lowered = url.to_lowercase();
    for (provider, markers) in PROVIDERS {
        if markers.iter().any(|m| lowered.contains(m)) {
            return Ok(*provider);
        }
    }
    let supported: Vec<&'static str> = PROVIDERS
        .iter()
        .flat_map(|(_, markers)| markers.iter().copied())
        .collect();
    anyhow::bail!(
        "Unsupported conversation URL: '{}'. Supported hosts: {}.",
        url,
        supported.join(", "),
    );
}

async fn inject_cookies_and_navigate(
    browser: &Browser,
    provider: Provider,
    cookie_data: &str,
    conversation_url: &str,
) -> Result<Page> {
    let cookies = parse_cookies(cookie_data)?;
    let page = browser.new_page(provider.bootstrap_url()).await?;
    install_stealth(&page).await?;
    page.set_cookies(cookies).await?;
    page.reload().await?;

    // Poll the landing page so Cloudflare's challenge JS has time to issue
    // cf_clearance before we hop to the conversation URL. Up to ~12s; if the
    // interstitial never clears, proceed and let the post-nav guard catch it.
    for i in 0..6 {
        human_pause(1_500, 2_500).await;
        if detect_interstitial(&page).await.is_none() {
            break;
        }
        if i == 5 {
            eprintln!(
                "{} landing page still showing interstitial after ~12s; proceeding to conversation URL anyway",
                "[fast-travel]".yellow()
            );
        }
    }

    let current_url = page.url().await.ok().flatten().unwrap_or_default();
    let lower = current_url.to_lowercase();
    if provider
        .auth_redirect_hosts()
        .iter()
        .any(|needle| lower.contains(needle))
    {
        anyhow::bail!(
            "Auth redirect detected (url='{}'). Cookie file for {} likely missing session cookies. Re-export via cookie-master-key on an authenticated {} tab.",
            current_url,
            provider.name(),
            provider.name(),
        );
    }

    eprintln!(
        "{} Navigating to {}",
        "[fast-travel]".magenta(),
        conversation_url
    );
    page.goto(conversation_url).await?;
    page.wait_for_navigation().await?;
    Ok(page)
}

async fn wait_for_conversation(page: &Page, provider: Provider) -> Result<()> {
    let script = provider.wait_script();
    for _ in 0..60 {
        let present: bool = page.evaluate(script).await?.into_value()?;
        if present {
            sleep(Duration::from_millis(2000)).await;
            return Ok(());
        }
        sleep(Duration::from_millis(500)).await;
    }
    anyhow::bail!(
        "Timed out waiting for {} conversation DOM. Selectors may have drifted, or auth didn't complete.",
        provider.name(),
    );
}

async fn extract_conversation(page: &Page, provider: Provider) -> Result<Vec<Message>> {
    page.evaluate("window.scrollTo(0, 0);").await?;
    sleep(Duration::from_millis(1500)).await;
    let result: ExtractionResult = page
        .evaluate(provider.extract_script())
        .await?
        .into_value()?;
    Ok(result.messages)
}

fn render_markdown(messages: &[Message]) {
    for (i, msg) in messages.iter().enumerate() {
        let header = match msg.role.as_str() {
            "user" => "## User",
            "model" => "## Model",
            other => {
                println!("## {}", capitalise(other));
                println!();
                println!("{}", msg.content);
                if i + 1 < messages.len() {
                    println!();
                }
                continue;
            }
        };
        println!("{header}\n\n{}", msg.content);
        if i + 1 < messages.len() {
            println!();
        }
    }
}

fn capitalise(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

fn resolve_profile_dir(args: &Args, label: &str) -> Result<ProfileDir> {
    let dir = match &args.profile_dir {
        Some(p) => ProfileDir(p.clone()),
        None => default_profile_dir(label)?,
    };
    ensure_profile_dir(&dir)?;
    Ok(dir)
}

/// Bundle returned by [`resolve_chrome_profile`]: the user-data-dir to pass to
/// chromium, and the profile-directory subdir name (e.g. "Profile 3").
struct ChromeProfile {
    user_data_dir: PathBuf,
    profile_subdir: String,
}

/// Look up an existing Chrome profile by its display name (the label shown in
/// the Chrome profile picker). Reads `<chrome-data-dir>/Local State` and walks
/// `profile.info_cache.<dir>.name`. Falls through to literal subdir match
/// (e.g. "Default", "Profile 1") if no display-name match.
fn resolve_chrome_profile(name: &str) -> Result<ChromeProfile> {
    let home = std::env::var("HOME").context("HOME is not set")?;

    // Try Chrome, Chromium (apt + snap layouts), and Chrome-for-Testing.
    // macOS path is different — follow-up for macOS users.
    let candidate_dirs = [
        // apt google-chrome
        PathBuf::from(&home).join(".config").join("google-chrome"),
        // apt chromium
        PathBuf::from(&home).join(".config").join("chromium"),
        // snap chromium (the canonical snap layout uses common/chromium directly,
        // not .config/chromium)
        PathBuf::from(&home).join("snap").join("chromium").join("common").join("chromium"),
        // chrome-for-testing
        PathBuf::from(&home).join(".config").join("google-chrome-for-testing"),
    ];

    // Collect all installations with a parseable Local State.
    let mut installations: Vec<(PathBuf, serde_json::Value)> = Vec::new();
    for dir in &candidate_dirs {
        let local_state_path = dir.join("Local State");
        if let Ok(content) = fs::read_to_string(&local_state_path) {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                installations.push((dir.clone(), json));
            }
        }
    }

    if installations.is_empty() {
        anyhow::bail!(
            "No Chrome/Chromium installation found. Tried {:?}",
            candidate_dirs
                .iter()
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>()
        );
    }

    // Search across all installations for the requested profile.
    for (user_data_dir, json) in &installations {
        let info_cache = json
            .get("profile")
            .and_then(|p| p.get("info_cache"))
            .and_then(|c| c.as_object());

        let mut available: Vec<(String, String)> = Vec::new();
        if let Some(cache) = info_cache {
            for (subdir, meta) in cache {
                let display = meta
                    .get("name")
                    .and_then(|n| n.as_str())
                    .unwrap_or("<unnamed>");
                available.push((subdir.clone(), display.to_string()));
            }
        }

        // Display-name match first.
        if let Some((subdir, _)) = available.iter().find(|(_, display)| display == name) {
            return Ok(ChromeProfile {
                user_data_dir: user_data_dir.clone(),
                profile_subdir: subdir.clone(),
            });
        }
        // Then literal subdir match.
        if let Some((subdir, _)) = available.iter().find(|(subdir, _)| subdir == name) {
            return Ok(ChromeProfile {
                user_data_dir: user_data_dir.clone(),
                profile_subdir: subdir.clone(),
            });
        }
    }

    // Not found in any installation — build error with the union of profiles.
    let mut error_lines = vec![
        format!("Chrome profile {name:?} not found."),
        "Searched these installations:".to_string(),
        String::new(),
    ];
    for (user_data_dir, json) in &installations {
        error_lines.push(format!("  {}:", user_data_dir.display()));

        let info_cache = json
            .get("profile")
            .and_then(|p| p.get("info_cache"))
            .and_then(|c| c.as_object());

        let mut available: Vec<(String, String)> = Vec::new();
        if let Some(cache) = info_cache {
            for (subdir, meta) in cache {
                let display = meta
                    .get("name")
                    .and_then(|n| n.as_str())
                    .unwrap_or("<unnamed>");
                available.push((subdir.clone(), display.to_string()));
            }
        }

        if available.is_empty() {
            error_lines.push("    (no profiles found)".to_string());
        } else {
            for (subdir, display) in available {
                error_lines.push(format!("    - {display:?} (subdir {subdir:?})"));
            }
        }
        error_lines.push(String::new()); // blank line between installations
    }
    if error_lines.last().is_some_and(|s| s.is_empty()) {
        error_lines.pop();
    }
    error_lines.push(format!(
        "To create a profile named {name:?}: open Chrome -> profile picker -> Add -> name it {name:?}, log into your LLMs there, then re-run this command."
    ));

    anyhow::bail!("{}", error_lines.join("\n"));
}

/// Detect whether Chrome is currently running with this user-data-dir by
/// looking for SingletonLock. Best-effort — false negatives are fine.
fn chrome_appears_running(user_data_dir: &Path) -> bool {
    user_data_dir.join("SingletonLock").exists()
        || user_data_dir.join("SingletonSocket").exists()
}

/// Open one persistent-profile Chromium with tabs for every requested provider.
/// Block on Enter so the user can log in and solve any CF challenges. The
/// resulting profile keeps cookies/state for all subsequent runs.
async fn run_init_all(args: &Args) -> Result<()> {
    let providers: Vec<Provider> = if args.init_providers.is_empty() {
        Provider::all().to_vec()
    } else {
        args.init_providers
            .iter()
            .map(|s| Provider::from_name(s))
            .collect::<Result<_>>()?
    };

    let opts = build_launch_opts(args, "shared/all-providers", true)?;
    eprintln!(
        "{} --init-all: opening visible Chromium",
        "[fast-travel]".magenta(),
    );
    if let Some(dir) = opts.profile_dir.as_ref() {
        eprintln!("    profile dir: {}", dir.display());
    }
    if !opts.extra_args.is_empty() {
        eprintln!("    extra args: {:?}", opts.extra_args);
    }

    let (mut browser, _handler) = launch(opts).await?;

    eprintln!(
        "{} Opening {} provider tabs:",
        "[fast-travel]".magenta(),
        providers.len()
    );
    for p in &providers {
        let url = p.bootstrap_url();
        eprintln!("    - {} -> {}", p.name(), url);
        // new_page opens a tab and navigates. Stealth is per-tab; install
        // before any cookied page load so CF's challenge JS sees the override.
        let page = browser.new_page(url).await?;
        install_stealth(&page).await?;
    }

    eprintln!();
    eprintln!(
        "{} Log into each tab. Solve any Cloudflare/auth challenges.",
        "[fast-travel]".green()
    );
    eprintln!(
        "{} When done, return here and press {} to close the browser",
        "[fast-travel]".green(),
        "ENTER".bold()
    );
    eprintln!(
        "{} (cookies and cf_clearance persist in the profile dir for future headless runs).",
        "[fast-travel]".green()
    );
    eprintln!();
    eprintln!(
        "{} Browser will auto-close after {}s, OR press ENTER here, OR `touch {}` from another terminal.",
        "[fast-travel]".green(),
        args.init_wait_secs,
        args.init_done_file
            .as_deref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "/tmp/ghostroute-init-done".to_string()),
    );
    print!("[ENTER to capture and exit early] ");
    io::stdout().flush().ok();

    // Three exit conditions, whichever fires first:
    //   - stdin Enter (interactive run)
    //   - sentinel file appears (background-friendly signal)
    //   - --init-wait-secs elapses (hard cap)
    let sentinel = args
        .init_done_file
        .clone()
        .unwrap_or_else(|| PathBuf::from("/tmp/ghostroute-init-done"));
    let _ = fs::remove_file(&sentinel); // start clean

    let stdin_task = tokio::task::spawn_blocking(move || {
        let mut line = String::new();
        io::stdin().lock().read_line(&mut line).ok();
    });
    let timeout_secs = args.init_wait_secs;
    let sentinel_clone = sentinel.clone();
    let sentinel_task = tokio::spawn(async move {
        loop {
            if sentinel_clone.exists() {
                return;
            }
            sleep(Duration::from_millis(800)).await;
        }
    });

    tokio::select! {
        _ = stdin_task => {
            eprintln!("\n{} stdin received, closing browser.", "[fast-travel]".green());
        }
        _ = sentinel_task => {
            eprintln!("\n{} Sentinel {} appeared, closing browser.", "[fast-travel]".green(), sentinel.display());
        }
        _ = sleep(Duration::from_secs(timeout_secs)) => {
            eprintln!("\n{} {}s timeout reached, closing browser.", "[fast-travel]".green(), timeout_secs);
        }
    }

    if let Err(e) = browser.close().await {
        eprintln!("{} Browser close warning: {}", "[fast-travel]".yellow(), e);
    }

    eprintln!("{} Done.", "[fast-travel]".green());
    if let Some(name) = args.chrome_profile.as_deref() {
        eprintln!(
            "{} State persisted in your Chrome profile {:?}. Reuse via:\n    --chrome-profile {:?}",
            "[fast-travel]".green(),
            name,
            name,
        );
    } else {
        let dir = resolve_profile_dir(args, "shared/all-providers")?;
        eprintln!(
            "{} Profile retained at {}\n    Reuse via: --profile-dir {}",
            "[fast-travel]".green(),
            dir.as_path().display(),
            dir.as_path().display(),
        );
    }
    Ok(())
}

/// Resolve `--profile-dir` / `--chrome-profile` precedence into a [`LaunchOpts`].
///
/// Precedence:
///   1. `--chrome-profile <name>` → use Chrome's user-data-dir + that profile subdir.
///   2. `--profile-dir <path>` → use the literal path (ghostroute-managed).
///   3. Default: `~/.cache/ghostroute/profiles/<label>`.
fn build_launch_opts(args: &Args, default_label: &str, visible: bool) -> Result<LaunchOpts> {
    if let Some(name) = args.chrome_profile.as_deref() {
        let cp = resolve_chrome_profile(name)?;
        if chrome_appears_running(&cp.user_data_dir) {
            anyhow::bail!(
                "Chrome appears to be running (SingletonLock present at {}). Close Chrome first, \
                 or use --profile-dir to launch against a separate ghostroute-managed profile.",
                cp.user_data_dir.display(),
            );
        }
        eprintln!(
            "{} Using Chrome profile {:?} at {} (subdir {:?})",
            "[fast-travel]".magenta(),
            name,
            cp.user_data_dir.display(),
            cp.profile_subdir,
        );
        return Ok(LaunchOpts {
            visible,
            profile_dir: Some(cp.user_data_dir),
            extra_args: vec![format!("--profile-directory={}", cp.profile_subdir)],
        });
    }

    let dir = resolve_profile_dir(args, default_label)?;
    Ok(LaunchOpts {
        visible,
        profile_dir: Some(dir.into_inner()),
        extra_args: vec![],
    })
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    if args.init_all {
        return run_init_all(&args).await;
    }

    let conversation_url = args
        .conversation_url
        .clone()
        .ok_or_else(|| anyhow::anyhow!("--conversation-url is required (or use --init-all)"))?;

    let provider = detect_provider(&conversation_url)?;
    eprintln!(
        "{} Provider: {} | URL: {}",
        "[fast-travel]".magenta(),
        provider.name(),
        conversation_url
    );

    if !args.dump_dom && provider.wait_script() == WAIT_STUB {
        anyhow::bail!(
            "Provider '{}' is recognised but its DOM selectors are still stubs. \
             Open fast-travel-cli/src/main.rs and replace WAIT_STUB / EXTRACT_STUB \
             in the Provider::{:?} arms. (Tip: --dump-dom captures the live DOM.)",
            provider.name(),
            provider,
        );
    }

    let cookie_path = cookie_file_path(provider.cookie_host())?;
    let cookie_data = load_cookie_data(&cookie_path)?;
    eprintln!(
        "{} Loaded {} cookies from {}",
        "[fast-travel]".magenta(),
        provider.name(),
        cookie_path.display()
    );

    let opts = build_launch_opts(
        &args,
        &format!("provider/{}", provider.name()),
        args.visible,
    )?;
    let (mut browser, _handler) = launch(opts).await?;

    if args.dump_dom {
        let result: Result<serde_json::Value> = async {
            let page =
                inject_cookies_and_navigate(&browser, provider, &cookie_data, &conversation_url)
                    .await?;

            let base = args.dump_settle_secs.max(1) * 1000;
            let min_ms = base * 80 / 100;
            let max_ms = base * 140 / 100;
            eprintln!(
                "{} --dump-dom: settling ~{}-{}ms before capture (jittered)",
                "[fast-travel]".magenta(),
                min_ms,
                max_ms,
            );
            human_pause(min_ms, max_ms).await;

            let _ = page.evaluate("window.scrollBy(0, 200);").await;
            human_pause(120, 380).await;
            let _ = page.evaluate("window.scrollBy(0, -200);").await;
            human_pause(120, 380).await;

            if let Some(reason) = detect_interstitial(&page).await {
                anyhow::bail!(
                    "Interstitial page detected: {}. Re-export cookies for {} (let the page \
                     fully load, then export via cookie-master-key) or use --init-all to \
                     solve the challenge interactively in a persistent profile.",
                    reason,
                    provider.name(),
                );
            }

            capture_dump_dom(&page).await
        }
        .await;

        if let Err(e) = browser.close().await {
            eprintln!("{} Browser close warning: {}", "[fast-travel]".yellow(), e);
        }

        let dump = result?;
        let pretty = serde_json::to_string_pretty(&dump)
            .unwrap_or_else(|_| "<serialisation failed>".to_string());
        match &args.dump_out {
            Some(path) => {
                fs::write(path, &pretty)?;
                eprintln!(
                    "{} Wrote DOM dump to {}",
                    "[fast-travel]".green(),
                    path.display()
                );
            }
            None => println!("{pretty}"),
        }
        return Ok(());
    }

    let run: Result<Vec<Message>> = async {
        let page =
            inject_cookies_and_navigate(&browser, provider, &cookie_data, &conversation_url)
                .await?;
        human_pause(400, 900).await;
        if let Some(reason) = detect_interstitial(&page).await {
            anyhow::bail!(
                "Interstitial page detected: {}. Refresh cookies for {} or use --init-all.",
                reason,
                provider.name(),
            );
        }
        wait_for_conversation(&page, provider).await?;
        extract_conversation(&page, provider).await
    }
    .await;

    if args.visible {
        eprintln!(
            "{} --visible: holding browser open 8s for manual inspection.",
            "[fast-travel]".yellow()
        );
        sleep(Duration::from_secs(8)).await;
    }

    if let Err(e) = browser.close().await {
        eprintln!("{} Browser close warning: {}", "[fast-travel]".yellow(), e);
    }

    let messages = run?;
    if messages.is_empty() {
        anyhow::bail!(
            "No messages extracted. Selectors may have drifted, or the conversation is empty."
        );
    }

    eprintln!(
        "{} Extracted {} messages — rendering to stdout.",
        "[fast-travel]".green(),
        messages.len()
    );
    render_markdown(&messages);
    Ok(())
}
