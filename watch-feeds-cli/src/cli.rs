use anyhow::{Context, Result};
use chrono::Utc;
use clap::ValueEnum;
use colored::Colorize;
use std::time::Duration;

use crate::db::{self, Feed};
use crate::feed;
use crate::sink::{self, Sink};

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

    let sink_writer = sink::for_target(sink_target);
    let total = scan_feeds_once(&conn, &feeds, &*sink_writer).await;

    eprintln!(
        "{} Scan complete. {} delta(s) emitted across {} feed(s).",
        "[watch-feeds]".cyan(),
        total,
        feeds.len()
    );
    Ok(())
}

/// Scan a slice of feeds once. Sink failures are logged per-feed but do NOT
/// abort the whole scan — one bad feed should not block the others. Returns
/// the total count of deltas successfully sunk.
async fn scan_feeds_once(
    conn: &rusqlite::Connection,
    feeds: &[Feed],
    sink_writer: &(dyn Sink + Send + Sync),
) -> usize {
    eprintln!(
        "{} Scanning {} feed(s)...",
        "[watch-feeds]".cyan(),
        feeds.len()
    );
    let mut total_deltas = 0_usize;
    for f in feeds {
        match feed::fetch_and_diff(f, conn).await {
            Ok(deltas) => {
                if !deltas.is_empty() {
                    eprintln!(
                        "{} {}: {} new entrie(s)",
                        "[watch-feeds]".green(),
                        f.title.as_deref().unwrap_or(&f.url),
                        deltas.len()
                    );
                    if let Err(e) = sink_writer.write_deltas(f, &deltas).await {
                        eprintln!(
                            "{} {}: sink failed — {:#}. Entries NOT recorded so they appear on next scan.",
                            "[watch-feeds]".red(),
                            f.title.as_deref().unwrap_or(&f.url),
                            e
                        );
                        continue;
                    }
                    if let Err(e) = db::record_entries(conn, f.id, &deltas) {
                        eprintln!(
                            "{} {}: record_entries failed — {:#}",
                            "[watch-feeds]".red(),
                            f.title.as_deref().unwrap_or(&f.url),
                            e
                        );
                        continue;
                    }
                    total_deltas += deltas.len();
                }
                if let Err(e) = db::touch_last_scanned(conn, f.id) {
                    eprintln!(
                        "{} {}: touch_last_scanned failed — {:#}",
                        "[watch-feeds]".red(),
                        f.title.as_deref().unwrap_or(&f.url),
                        e
                    );
                }
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
    total_deltas
}

pub async fn run_watch(state_db: Option<&str>, sink_target: SinkTarget) -> Result<()> {
    let conn = db::open(state_db).context("Failed to open feed state database")?;
    let sink_writer = sink::for_target(sink_target);

    eprintln!(
        "{} Watch mode started. Ctrl-C to exit cleanly.",
        "[watch-feeds]".cyan()
    );

    loop {
        let feeds = db::list_feeds(&conn).context("Failed to read feed list")?;
        if feeds.is_empty() {
            eprintln!(
                "{} No feeds configured. Sleeping 1 hour before re-checking.",
                "[watch-feeds]".yellow()
            );
            if sleep_or_exit(Duration::from_secs(3600)).await {
                break;
            }
            continue;
        }

        let now = Utc::now();
        let due_feeds: Vec<Feed> = feeds
            .iter()
            .filter(|f| match f.last_scanned_at {
                Some(t) => (now - t).num_minutes() >= f.cadence_minutes,
                None => true,
            })
            .cloned()
            .collect();

        if !due_feeds.is_empty() {
            scan_feeds_once(&conn, &due_feeds, &*sink_writer).await;
        }

        // Calculate sleep until the next feed becomes due. Floor at 60s so a
        // misconfigured 0-cadence feed cannot busy-loop. Cap at the cadence
        // of the soonest feed so newly-added feeds get picked up promptly.
        let now = Utc::now();
        let next_due_minutes: i64 = feeds
            .iter()
            .map(|f| {
                let elapsed = match f.last_scanned_at {
                    Some(t) => (now - t).num_minutes(),
                    None => f.cadence_minutes,
                };
                (f.cadence_minutes - elapsed).max(0)
            })
            .min()
            .unwrap_or(60);

        let sleep_seconds = ((next_due_minutes * 60).max(60)) as u64;
        eprintln!(
            "{} Next due in {} min. Sleeping...",
            "[watch-feeds]".cyan(),
            sleep_seconds / 60
        );

        if sleep_or_exit(Duration::from_secs(sleep_seconds)).await {
            break;
        }
    }

    eprintln!("{} Watch mode exiting.", "[watch-feeds]".cyan());
    Ok(())
}

/// Sleep for `dur` or until SIGINT/SIGTERM. Returns `true` if interrupted.
async fn sleep_or_exit(dur: Duration) -> bool {
    tokio::select! {
        _ = tokio::time::sleep(dur) => false,
        _ = tokio::signal::ctrl_c() => {
            eprintln!("\n{} Received Ctrl-C, finishing current iteration...", "[watch-feeds]".yellow());
            true
        }
    }
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
