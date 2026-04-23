use clap::{Parser, ValueEnum, Subcommand};
use chromiumoxide::browser::{Browser, BrowserConfig};
use chromiumoxide::cdp::browser_protocol::network::CookieParam;
use futures::StreamExt;
use std::env;
use std::path::PathBuf;
use std::fs;
use colored::*; // Add `cargo add colored` to your project!
use chrono::Local; // Add `cargo add chrono` to your project!
use rand::{Rng, RngExt};
use serde::{Deserialize, Serialize};

// Define our specific extraction realms
#[derive(ValueEnum, Clone, Debug)]
enum Provider {
    Chatgpt,
    Claude,
    Gemini,
    Generic,
}

#[derive(Parser, Debug)]
#[command(author, version, about = "Extracts and indexes LLM conversations via browser automation", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Scrape the sidebar to find and index all recent conversations
    Sync {
        #[arg(short, long, value_enum)]
        provider: Provider,
    },
    /// Extract and save a specific conversation
    Extract {
        #[arg(short, long, value_enum)]
        provider: Provider,
        #[arg(short, long)]
        url: String,
    },
    /// Search your local index for a conversation
    Search {
        /// The keyword to search for in titles
        query: String,
    },
}



// Assumes your cookie file is an array of objects exported from a Chrome extension
async fn inject_cookies(page: &chromiumoxide::Page, cookie_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("{} Unlocking gates with cookies from {}...", "[Vault]".magenta(), cookie_path);
    
    let cookie_data = fs::read_to_string(cookie_path)?;
    let parsed_cookies: serde_json::Value = serde_json::from_str(&cookie_data)?;

    if let Some(cookies_array) = parsed_cookies.as_array() {
        for c in cookies_array {
            let name = c["name"].as_str().unwrap_or("").to_string();
            let value = c["value"].as_str().unwrap_or("").to_string();
            let domain = c["domain"].as_str().unwrap_or("").to_string();
            
            // Build the CDP Cookie parameter
            let mut param = CookieParam::new(name, value);
            param.domain = Some(domain);
            
            // Inject it into the current page context
            page.set_cookie(param).await?;
        }
    }
    Ok(())
}

fn get_index_path() -> PathBuf {
    let home_dir = env::var("HOME").unwrap_or_default();
    let index_dir = PathBuf::from(&home_dir).join(".claude").join("fast-travel");
    if !index_dir.exists() {
        fs::create_dir_all(&index_dir).unwrap();
    }
    index_dir.join("index.json")
}

fn update_master_index(new_entries: Vec<LoreIndex>) {
    let index_path = get_index_path();
    
    // Load existing index or create new
    let mut current_index: Vec<LoreIndex> = if index_path.exists() {
        let data = fs::read_to_string(&index_path).unwrap_or_default();
        serde_json::from_str(&data).unwrap_or_default()
    } else {
        Vec::new()
    };

    // Merge new entries (avoiding URL duplicates)
    for new_entry in new_entries {
        if !current_index.iter().any(|e| e.url == new_entry.url) {
            current_index.push(new_entry);
        }
    }

    // Save back to disk
    let json_output = serde_json::to_string_pretty(&current_index).unwrap();
    fs::write(&index_path, json_output).expect("Failed to write master index.");
    
    eprintln!("{} Indexed {} conversations locally.", "[Library]".magenta(), current_index.len());
}

fn search_index(query: &str) {
    let index_path = get_index_path();
    if !index_path.exists() {
        eprintln!("{} No index found. Run 'sync' first!", "[Error]".red());
        return;
    }

    let data = fs::read_to_string(&index_path).unwrap_or_default();
    let current_index: Vec<LoreIndex> = serde_json::from_str(&data).unwrap_or_default();

    let query_lower = query.to_lowercase();
    let mut found = false;

    println!("=== 🔍 Search Results for '{}' ===", query);
    for entry in current_index {
        if entry.title.to_lowercase().contains(&query_lower) {
            println!("- [{}] {}\n  URL: {}", entry.provider, entry.title, entry.url);
            if let Some(path) = &entry.local_path {
                println!("  Local Copy: {}", path);
            }
            println!();
            found = true;
        }
    }

    if !found {
        println!("No matching conversations found.");
    }
}

async fn sync_conversations(page: &chromiumoxide::Page, provider: &Provider) -> Result<(), Box<dyn std::error::Error>> {
    let url = match provider {
        Provider::Gemini => "https://gemini.google.com/app",
        Provider::Chatgpt => "https://chatgpt.com",
        Provider::Claude => "https://claude.ai/chats",
        Provider::Generic => return Err("Cannot sync generic providers.".into()),
    };

    eprintln!("{} Pinging radar at {}...", "[Sync]".cyan(), url);
    
    // Inject cookies based on provider here (using the function we built earlier)
    // inject_cookies(page, &cookie_path).await?;

    page.goto(url).await?;
    page.wait_for_navigation().await?;
    
    // Wait for the sidebar to load and hydrate
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;

    // JavaScript to extract Sidebar URLs
    let js_extractor = match provider {
        Provider::Gemini => r#"
            (() => {
                let chats = [];
                // Gemini sidebar chats usually are anchor tags linking to /app/
                document.querySelectorAll('a[href^="/app/"]').forEach(node => {
                    let title = node.innerText || "Untitled Chat";
                    if (title.trim().length > 0 && title !== "New chat") {
                        chats.push({ title: title.trim(), url: node.href });
                    }
                });
                return chats;
            })();
        "#,
        // Add ChatGPT and Claude logic here later
        _ => "(() => { return []; })();"
    };

    let res = page.evaluate(js_extractor).await?;
    
    // Parse the JSON array of objects
    let fetched_chats: Vec<serde_json::Value> = if let Some(val) = res.value {
        serde_json::from_value(val).unwrap_or_default()
    } else {
        Vec::new()
    };

    // Build our Index Objects
    let mut new_entries = Vec::new();
    for chat in fetched_chats {
        if let (Some(title), Some(url)) = (chat.get("title").and_then(|t| t.as_str()), chat.get("url").and_then(|u| u.as_str())) {
            new_entries.push(LoreIndex {
                title: title.to_string(),
                url: url.to_string(),
                provider: format!("{:?}", provider),
                local_path: None,
            });
        }
    }

    // Save to master index
    update_master_index(new_entries);
    
    eprintln!("{} Radar sync complete!", "[Success]".green());
    Ok(())
}



async fn extract_lore(page: &chromiumoxide::Page, provider: &Provider, url: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    // 1. INJECT COOKIES (Only for Gemini/authenticated routes)
    if matches!(provider, Provider::Gemini) {
        // Adjust this path to wherever your global inventory is!
        let home_dir = env::var("HOME").unwrap_or_default();
        let cookie_path = format!("{}/.claude/cookie-configs/gemini.google.com-cookies.json", home_dir);
        inject_cookies(page, &cookie_path).await?;
    }
    eprintln!("{} Warping to {}...", "[Fast-Travel]".cyan(), url);
    page.goto(url).await?;
    page.wait_for_navigation().await?;
    
    // 2. THE TOURIST MECHANIC (Human-like behavior)
    eprintln!("{} Simulating human reading patterns...", "[Stealth]".yellow());
    
    // Initial load pause (Humans take a second to orient themselves)
    let initial_delay = rand::rng().random_range(2500..4500);
    tokio::time::sleep(std::time::Duration::from_millis(initial_delay)).await;

    // Simulate scrolling down to trigger any lazy-loaded UI elements
    for _ in 0..3 {
        let scroll_js = "window.scrollBy(0, window.innerHeight * 0.75);";
        page.evaluate(scroll_js).await?;
        
        // Random pause between scrolls (reading time)
        let scroll_delay = rand::rng().random_range(800..2000);
        tokio::time::sleep(std::time::Duration::from_millis(scroll_delay)).await;
    }

    // Allow React/Next.js to hydrate
    tokio::time::sleep(std::time::Duration::from_secs(4)).await;

    let js_extractor = match provider {
        Provider::Gemini => r#"
            (() => {
                let messages = [];
                // Gemini frequently uses these custom tags and classes for its conversation UI
                let nodes = document.querySelectorAll('user-query, model-response, .user-query-content, .model-response-text');
                
                nodes.forEach(node => {
                    let text = node.innerText || "";
                    if (text.trim().length > 0) {
                        // Determine role based on tag name or class
                        let isUser = node.tagName.toLowerCase().includes('user') || node.className.includes('user');
                        let role = isUser ? 'USER' : 'GEMINI';
                        messages.push(`**Role: ${role}**\n\n${text}`);
                    }
                });

                return messages;
            })();
        "#,
        Provider::Chatgpt => r#"
            (() => {
                let messages = [];
                // ChatGPT specifically uses these data attributes
                document.querySelectorAll('div[data-message-author-role]').forEach(node => {
                    let role = node.getAttribute('data-message-author-role');
                    let text = node.innerText || "";
                    if (text.trim().length > 0) {
                        messages.push(`**Role: ${role.toUpperCase()}**\n\n${text}`);
                    }
                });
                return messages;
            })();
        "#,
        Provider::Claude => r#"
            (() => {
                let messages = [];
                // Claude generally alternates user/assistant. We look for their standard text blocks.
                // Note: Anthropic changes these classes frequently, you may need to inspect the DOM!
                document.querySelectorAll('.font-user-message, .font-claude-message').forEach(node => {
                    let role = node.className.includes('user') ? 'USER' : 'CLAUDE';
                    let text = node.innerText || "";
                    if (text.trim().length > 0) {
                        messages.push(`**Role: ${role}**\n\n${text}`);
                    }
                });
                return messages;
            })();
        "#,
        Provider::Generic => r#"
            (() => {
                let messages = [];
                document.querySelectorAll('article, .prose, div[dir="auto"]').forEach(node => {
                    let text = node.innerText || "";
                    if (text.trim().length > 0) messages.push(text);
                });
                return messages;
            })();
        "#,
    };

    let res = page.evaluate(js_extractor).await?;
    
    let messages: Vec<String> = if let Some(val) = res.value() {
        serde_json::from_value(val.clone()).unwrap_or_default()
    } else {
        Vec::new()
    };

    Ok(messages)
}

pub fn save_lore_compendium(project_root: &PathBuf, url: &str, messages: Vec<String>) -> String {
    let fast_travel_dir = project_root.join(".claude").join("fast-travel");
    if !fast_travel_dir.exists() {
        fs::create_dir_all(&fast_travel_dir).expect("Failed to build fast-travel directory!");
    }

    let timestamp = Local::now().format("%Y-%m-%d_%H-%M-%S").to_string();
    let file_name = format!("chat_{}.md", timestamp);
    let file_path = fast_travel_dir.join(&file_name);

    // 1. Build the YAML Front Matter
    let mut md_content = String::new();
    md_content.push_str("---\n");
    md_content.push_str(&format!("source_url: \"{}\"\n", url));
    md_content.push_str(&format!("extracted_on: \"{}\"\n", Local::now().format("%Y-%m-%d %H:%M:%S")));
    md_content.push_str(&format!("message_count: {}\n", messages.len()));
    md_content.push_str("---\n\n");

    // 2. Append the Messages
    for msg in messages {
        md_content.push_str(&msg);
        md_content.push_str("\n\n---\n\n");
    }

    fs::write(&file_path, md_content).expect("Failed to write Markdown file");

    // 3. Update the Index/Summary File
    let index_path = fast_travel_dir.join("summary.md");
    let mut index_content = if index_path.exists() {
        fs::read_to_string(&index_path).unwrap_or_default()
    } else {
        "# 📚 Fast-Travel Lore Compendium\n\n".to_string()
    };

    index_content.push_str(&format!("- **[{}]** Extracted {} messages from: {}\n", 
        timestamp, 
        file_name, 
        url
    ));

    fs::write(&index_path, index_content).expect("Failed to update index file");

    // Return the path of the summary file for the handoff
    index_path.to_string_lossy().to_string()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Search { query } => {
            search_index(query);
            Ok(())
        }
        Commands::Sync { provider } => {
            let (mut browser, mut handler) = chromiumoxide::browser::Browser::launch(
                chromiumoxide::browser::BrowserConfig::builder().with_head().build()?
            ).await?;
            let _handle = tokio::spawn(async move { while let Some(h) = handler.next().await { if h.is_err() { break; } } });
            
            let page = browser.new_page("about:blank").await?;
            sync_conversations(&page, provider).await?;
            Ok(())
        }
        Commands::Extract { provider, url } => {
            // Your existing extraction logic goes here. 
            // *Pro Tip: Once extraction is done, update the master index to set `local_path` 
            // for this specific URL so your search command knows it's downloaded!
            eprintln!("Extracting from {}", url);
            Ok(())
        }
    }

    // 1. Boot the Browser (Headless is fine for scraping!)
    let (mut browser, mut handler) = Browser::launch(
        BrowserConfig::builder().with_head().build()? 
    ).await?;

    // Background process for WebSockets
    let _handle = tokio::spawn(async move {
        while let Some(h) = handler.next().await {
            if h.is_err() { break; }
        }
    });

    let page = browser.new_page("about:blank").await?;

    // 2. Extract Data
    let messages = extract_lore(&page, &args.provider, &args.url).await?;
    
    if messages.is_empty() {
        eprintln!("{} Failed to extract lore. The DOM selector might be outdated.", "[Error]".red());
        return Ok(());
    }

    // 3. Save the Grimoire
    // Determine Git root dynamically like in ask-grok-cli
    let project_root = env::current_dir().unwrap_or_else(|_| PathBuf::from(".")); 
    
    // Assume you pasted the `save_lore_compendium` function here
    // let summary_path = save_lore_compendium(&project_root, &args.url, messages);
    let summary_path = "path/to/mock/summary.md"; // Placeholder for compiled logic

    // 4. The MCP Handoff
    eprintln!("{} Data archived successfully.", "[Success]".green());
    println!("FAST_TRAVEL_COMPLETE: Extracted conversation from {}. Please read the index at: {}", args.url, summary_path);

    Ok(())
}