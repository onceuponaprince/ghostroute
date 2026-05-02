use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use feed_rs::parser;
use rusqlite::Connection;

use crate::db::{self, Entry, Feed};

/// GitHub's *.atom endpoints return 406 Not Acceptable when called without an
/// explicit Accept header that names a feed mime-type. The wildcard `*/*` that
/// reqwest sends by default is not enough. Listing every feed format we want
/// to support keeps the same client useful against non-GitHub sources.
const ACCEPT_HEADER: &str =
    "application/atom+xml, application/rss+xml, application/feed+json, application/json, application/xml;q=0.9, text/xml;q=0.9, */*;q=0.5";

/// Fetch a feed's current state and return only the entries we have NOT seen
/// before. Entries are NOT yet recorded in the DB by this function — the
/// caller decides whether to commit them (typically: only after the sink has
/// successfully consumed the deltas, so a sink failure does not silently drop
/// the entries from the next run's diff).
pub async fn fetch_and_diff(feed: &Feed, conn: &Connection) -> Result<Vec<Entry>> {
    let client = reqwest::Client::builder()
        .user_agent(concat!(
            "watch-feeds-cli/",
            env!("CARGO_PKG_VERSION"),
            " (+ghostroute monorepo)"
        ))
        .build()
        .context("Failed to build HTTP client")?;

    let body = client
        .get(&feed.url)
        .header(reqwest::header::ACCEPT, ACCEPT_HEADER)
        .send()
        .await
        .with_context(|| format!("HTTP fetch failed for {}", feed.url))?
        .error_for_status()
        .with_context(|| format!("Non-2xx response from {}", feed.url))?
        .bytes()
        .await
        .with_context(|| format!("Body read failed for {}", feed.url))?;

    let parsed = parser::parse(&body[..])
        .with_context(|| format!("Feed parse failed for {}", feed.url))?;

    let mut deltas = Vec::new();
    for item in parsed.entries {
        let guid = item.id.clone();
        if guid.is_empty() {
            continue;
        }
        if db::entry_already_seen(conn, feed.id, &guid)? {
            continue;
        }
        let title = item
            .title
            .map(|t| t.content)
            .unwrap_or_else(|| "(untitled)".to_string());
        let url = item
            .links
            .first()
            .map(|l| l.href.clone())
            .unwrap_or_default();
        let published_at: Option<DateTime<Utc>> = item.published.or(item.updated);
        let summary = item.summary.map(|s| s.content);
        deltas.push(Entry {
            guid,
            title,
            url,
            published_at,
            summary,
        });
    }
    Ok(deltas)
}
