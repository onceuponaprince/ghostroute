use chromiumoxide::cdp::browser_protocol::network::CookieParam;
use chromiumoxide::{Browser, BrowserConfig, Page};
use clap::Parser;
use colored::Colorize;
use futures::StreamExt;
use serde::Deserialize;
use std::env;
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use tokio::time::sleep;

type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;

#[derive(Parser, Debug)]
#[command(
    name = "fast-travel-cli",
    about = "Carry Gemini conversations into Claude without blowing the context budget"
)]
struct Args {
    /// Gemini conversation URL (e.g. https://gemini.google.com/app/<id>)
    #[arg(long)]
    conversation_url: String,
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

fn resolve_cookie_path() -> Result<PathBuf> {
    let home = env::var("HOME")?;
    Ok(PathBuf::from(home)
        .join(".claude")
        .join("cookie-configs")
        .join("gemini.google.com-cookies.json"))
}

fn load_cookies() -> Result<String> {
    let path = resolve_cookie_path()?;
    let data = fs::read_to_string(&path).map_err(|e| {
        format!(
            "Failed to read cookie file {}: {}. Export cookies from an authenticated Gemini tab via cookie-master-key.",
            path.display(),
            e
        )
    })?;
    eprintln!(
        "{} Loaded cookies from {}",
        "[fast-travel]".magenta(),
        path.display()
    );
    Ok(data)
}

async fn launch_browser() -> Result<(Browser, tokio::task::JoinHandle<()>)> {
    let config = BrowserConfig::builder()
        .arg("--no-sandbox")
        .build()
        .map_err(|e| format!("Failed to build BrowserConfig: {}", e))?;
    let (browser, mut handler) = Browser::launch(config).await?;
    let handle = tokio::spawn(async move {
        while handler.next().await.is_some() {}
    });
    Ok((browser, handle))
}

async fn inject_cookies_and_navigate(
    browser: &Browser,
    cookie_data: &str,
    conversation_url: &str,
) -> Result<Page> {
    let cookies: Vec<CookieParam> = serde_json::from_str(cookie_data)
        .map_err(|e| format!("Cookie file is not valid CDP cookie JSON: {}", e))?;

    // CDP rejects set_cookies on about:blank. Navigate to a real domain first.
    let page = browser.new_page("https://gemini.google.com").await?;
    page.set_cookies(cookies).await?;
    page.reload().await?;

    let current_url = page.url().await.ok().flatten().unwrap_or_default();
    let lower = current_url.to_lowercase();
    if lower.contains("accounts.google.com") || lower.contains("/signin") || lower.contains("/login") {
        return Err(format!(
            "Auth redirect detected (url='{}'). Cookie file likely missing Google session cookies (SID, SSID, __Secure-*). Re-export via cookie-master-key on an authenticated Gemini tab.",
            current_url
        )
        .into());
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

async fn wait_for_conversation(page: &Page) -> Result<()> {
    // Poll for Gemini's conversation DOM. Timeout ~30s.
    for _ in 0..60 {
        let present: bool = page
            .evaluate(
                r#"
                (() => {
                    return document.querySelector('user-query, model-response, .user-query-content, .model-response-text') !== null;
                })();
                "#,
            )
            .await?
            .into_value()?;
        if present {
            // Hydration grace period so later messages render before extraction.
            sleep(Duration::from_millis(2000)).await;
            return Ok(());
        }
        sleep(Duration::from_millis(500)).await;
    }
    Err("Timed out waiting for Gemini conversation DOM. Selectors may have drifted, or auth didn't complete.".into())
}

async fn extract_conversation(page: &Page) -> Result<Vec<Message>> {
    // Scroll to top once so virtualised older messages render before we read them.
    page.evaluate("window.scrollTo(0, 0);").await?;
    sleep(Duration::from_millis(1500)).await;

    let result: ExtractionResult = page
        .evaluate(
            r#"
            (() => {
                const messages = [];
                const nodes = document.querySelectorAll(
                    'user-query, model-response, .user-query-content, .model-response-text'
                );
                nodes.forEach(node => {
                    const tag = (node.tagName || '').toLowerCase();
                    const cls = typeof node.className === 'string' ? node.className : '';
                    const isUser = tag.includes('user') || cls.includes('user');
                    const role = isUser ? 'user' : 'model';
                    const content = (node.innerText || '').trim();
                    if (content.length > 0) {
                        messages.push({ role, content });
                    }
                });
                return { messages };
            })();
            "#,
        )
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
        println!("{}", header);
        println!();
        println!("{}", msg.content);
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

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    eprintln!(
        "{} Conversation URL: {}",
        "[fast-travel]".magenta(),
        args.conversation_url
    );

    let cookie_data = load_cookies()?;
    let (mut browser, _handler) = launch_browser().await?;

    let run: Result<Vec<Message>> = async {
        let page =
            inject_cookies_and_navigate(&browser, &cookie_data, &args.conversation_url).await?;
        wait_for_conversation(&page).await?;
        extract_conversation(&page).await
    }
    .await;

    if let Err(e) = browser.close().await {
        eprintln!(
            "{} Browser close warning: {}",
            "[fast-travel]".yellow(),
            e
        );
    }

    let messages = run?;
    if messages.is_empty() {
        return Err(
            "No messages extracted. Selectors may have drifted, or the conversation is empty."
                .into(),
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
