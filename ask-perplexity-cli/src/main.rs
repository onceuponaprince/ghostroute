use anyhow::Result;
use ask_perplexity_cli::automation::scrape::scrape_once;
use ask_perplexity_cli::browser::bootstrap_browser;
use ask_perplexity_cli::cli::args::Args;
use ask_perplexity_cli::config::cookies::{cookie_file_path, read_cookie_json};
use ask_perplexity_cli::parse::{parse, ParseOptions};
use ask_perplexity_cli::types::PerplexityResult;
use clap::Parser;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // If --deep, emit a synthetic correlation jobId to stderr up front.
    let job_id = if args.deep {
        let id = Uuid::new_v4().to_string();
        eprintln!("[job] {id}");
        Some(id)
    } else {
        None
    };

    let cookie_path = cookie_file_path()?;
    let cookie_json = read_cookie_json(&cookie_path)?;
    let (mut browser, page) = bootstrap_browser(&cookie_json).await?;

    let on_progress: Option<Box<dyn FnMut(String) + Send>> = if args.deep {
        Some(Box::new(|text: String| {
            eprintln!("[progress] {text}");
        }))
    } else {
        None
    };

    let tool = if args.deep { Some("deep-research") } else { None };
    let (html, final_url) = scrape_once(
        &page,
        &args.prompt,
        args.model.as_str(),
        tool,
        args.focus.as_str(),
        args.thread.as_deref(),
        on_progress,
    )
    .await?;

    // Close the browser before parsing (keeps peak memory lower).
    drop(page);
    browser.close().await.ok();

    let mut result: PerplexityResult = parse(
        &html,
        ParseOptions {
            url: Some(&final_url),
            deep: args.deep,
            raw: args.raw,
        },
    )?;
    result.job_id = job_id;

    // Pretty-print the JSON for human readability; still pipe-safe.
    let json = serde_json::to_string_pretty(&result)?;
    println!("{json}");
    Ok(())
}
