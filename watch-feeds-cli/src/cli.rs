use anyhow::{Context, Result};
use clap::ValueEnum;
use colored::Colorize;

use crate::db;
use crate::feed;
use crate::sink;

#[derive(Copy, Clone, Debug, ValueEnum)]
pub enum SinkTarget {
    /// Write deltas as markdown event files into $BORAI_INBOX_PATH/events/.
    BoraiInbox,
    /// Print deltas as JSON-lines to stdout. Caller pipes wherever.
    Stdout,
    /// Discard deltas. Useful for first-run priming of last-seen state.
    Null,
}

pub async fn run_scan(state_db: Option<&str>, sink_target: SinkTarget) -> Result<()> {
    let conn = db::open(state_db).context("Failed to open feed state database")?;
    let feeds = db::list_feeds(&conn).context("Failed to read feed list")?;
    if feeds.is_empty() {
        eprintln!(
            "{} No feeds configured. Add one with: watch-feeds-cli add-feed <url>",
            "[watch-feeds]".yellow()
        );
        return Ok(());
    }

    eprintln!(
        "{} Scanning {} feed(s)...",
        "[watch-feeds]".cyan(),
        feeds.len()
    );

    let sink_writer = sink::for_target(sink_target);
    let mut total_deltas = 0_usize;

    for f in &feeds {
        match feed::fetch_and_diff(f, &conn).await {
            Ok(deltas) => {
                if !deltas.is_empty() {
                    eprintln!(
                        "{} {}: {} new entrie(s)",
                        "[watch-feeds]".green(),
                        f.title.as_deref().unwrap_or(&f.url),
                        deltas.len()
                    );
                    total_deltas += deltas.len();
                    sink_writer.write_deltas(f, &deltas).await?;
                    db::record_entries(&conn, f.id, &deltas)?;
                }
                db::touch_last_scanned(&conn, f.id)?;
            }
            Err(e) => {
                eprintln!(
                    "{} {}: fetch failed — {:#}",
                    "[watch-feeds]".red(),
                    f.title.as_deref().unwrap_or(&f.url),
                    e
                );
            }
        }
    }

    eprintln!(
        "{} Scan complete. {} delta(s) emitted across {} feed(s).",
        "[watch-feeds]".cyan(),
        total_deltas,
        feeds.len()
    );
    Ok(())
}

pub async fn run_watch(_state_db: Option<&str>, _sink_target: SinkTarget) -> Result<()> {
    anyhow::bail!("watch mode not yet implemented — use scan + external scheduler for now")
}

pub fn run_add_feed(
    state_db: Option<&str>,
    url: &str,
    cadence_minutes: i64,
    title: Option<&str>,
) -> Result<()> {
    let conn = db::open(state_db)?;
    let id = db::add_feed(&conn, url, title, cadence_minutes)?;
    println!(
        "{} Feed #{} added: {} (cadence: {} min)",
        "[watch-feeds]".green(),
        id,
        url,
        cadence_minutes
    );
    Ok(())
}

pub fn run_list_feeds(state_db: Option<&str>) -> Result<()> {
    let conn = db::open(state_db)?;
    let feeds = db::list_feeds(&conn)?;
    if feeds.is_empty() {
        println!("{} No feeds configured.", "[watch-feeds]".yellow());
        return Ok(());
    }
    for f in &feeds {
        let last_scanned = f
            .last_scanned_at
            .map(|t| t.to_rfc3339())
            .unwrap_or_else(|| "never".to_string());
        println!(
            "#{:<4} {:<60} cadence={:>5}min last_scanned={}",
            f.id,
            f.title.as_deref().unwrap_or(&f.url),
            f.cadence_minutes,
            last_scanned
        );
    }
    Ok(())
}

pub fn run_remove_feed(state_db: Option<&str>, target: &str) -> Result<()> {
    let conn = db::open(state_db)?;
    let removed = db::remove_feed(&conn, target)?;
    if removed {
        println!("{} Feed removed: {}", "[watch-feeds]".green(), target);
    } else {
        eprintln!("{} No feed matched: {}", "[watch-feeds]".yellow(), target);
    }
    Ok(())
}
