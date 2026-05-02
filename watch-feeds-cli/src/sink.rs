use anyhow::{Context, Result};
use chrono::Utc;
use std::env;
use std::fs;
use std::path::PathBuf;

use crate::cli::SinkTarget;
use crate::db::{Entry, Feed};

pub trait Sink {
    fn write_deltas<'a>(
        &'a self,
        feed: &'a Feed,
        deltas: &'a [Entry],
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + 'a>>;
}

pub fn for_target(target: SinkTarget) -> Box<dyn Sink + Send + Sync> {
    match target {
        SinkTarget::BoraiInbox => Box::new(BoraiInboxSink),
        SinkTarget::Stdout => Box::new(StdoutSink),
        SinkTarget::Null => Box::new(NullSink),
    }
}

/// BorAI inbox sink. Writes one batched `source_update` event per scanned
/// feed (containing all the new entries from that scan) into the inbox's
/// `events/` directory.
///
/// Path resolution: $BORAI_INBOX_PATH if set, else `~/code/borai/inbox`. The
/// schema reference documents `ops/borai-inbox/` but the actual repo lays it
/// out as `inbox/` at the top level — sink follows reality, not the doc.
///
/// The `_index.json` schema field is described in the staging-schema doc but
/// the live inbox does not maintain one (the consumer daemon reads files
/// directly per `~/code/borai/inbox/README.md`). Sink writes only the event
/// file and trusts whoever is consuming to discover it.
struct BoraiInboxSink;

impl BoraiInboxSink {
    fn inbox_path() -> Result<PathBuf> {
        if let Ok(p) = env::var("BORAI_INBOX_PATH") {
            return Ok(PathBuf::from(p));
        }
        let home = env::var("HOME").context("HOME env var not set")?;
        Ok(PathBuf::from(home).join("code").join("borai").join("inbox"))
    }

    fn slugify(input: &str) -> String {
        let mut out = String::with_capacity(input.len());
        let mut prev_dash = false;
        for ch in input.chars() {
            if ch.is_ascii_alphanumeric() {
                out.push(ch.to_ascii_lowercase());
                prev_dash = false;
            } else if !prev_dash && !out.is_empty() {
                out.push('-');
                prev_dash = true;
            }
        }
        while out.ends_with('-') {
            out.pop();
        }
        if out.is_empty() {
            out.push_str("untitled");
        }
        out
    }
}

impl Sink for BoraiInboxSink {
    fn write_deltas<'a>(
        &'a self,
        feed: &'a Feed,
        deltas: &'a [Entry],
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move {
            if deltas.is_empty() {
                return Ok(());
            }

            let inbox = Self::inbox_path()?;
            let events_dir = inbox.join("events");
            fs::create_dir_all(&events_dir).with_context(|| {
                format!("Failed to create inbox events dir at {}", events_dir.display())
            })?;

            let now = Utc::now();
            let timestamp = now.format("%Y-%m-%dT%H-%M-%SZ").to_string();
            let feed_slug = Self::slugify(feed.title.as_deref().unwrap_or(&feed.url));
            let filename = format!("{timestamp}_source-update_{feed_slug}.md");
            let path = events_dir.join(&filename);

            let expires_at = now + chrono::Duration::days(14);

            let mut body = String::new();
            body.push_str("---\n");
            body.push_str("event_type: source_update\n");
            body.push_str("product: portfolio\n");
            body.push_str(&format!("timestamp: {}\n", now.to_rfc3339()));
            body.push_str("source_skill: watch-feeds-cli\n");
            body.push_str("priority: normal\n");
            body.push_str("requires_approval: false\n");
            body.push_str("approval_status: n/a\n");
            body.push_str(&format!("expires_at: {}\n", expires_at.to_rfc3339()));
            body.push_str(&format!("feed_url: {}\n", feed.url));
            if let Some(t) = &feed.title {
                body.push_str(&format!("feed_title: {t:?}\n"));
            }
            body.push_str(&format!("entry_count: {}\n", deltas.len()));
            body.push_str("---\n\n");

            body.push_str(&format!(
                "## {}: {} new entrie(s)\n\n",
                feed.title.as_deref().unwrap_or(&feed.url),
                deltas.len()
            ));

            for d in deltas {
                body.push_str(&format!("### {}\n\n", d.title));
                if let Some(pub_at) = d.published_at {
                    body.push_str(&format!("*Published: {}*\n\n", pub_at.to_rfc3339()));
                }
                if !d.url.is_empty() {
                    body.push_str(&format!("Read: {}\n\n", d.url));
                }
                if let Some(summary) = &d.summary {
                    let summary = summary.trim();
                    if !summary.is_empty() {
                        let preview: String = summary.chars().take(500).collect();
                        body.push_str(&format!("> {preview}\n\n"));
                    }
                }
            }

            body.push_str(&format!(
                "\n*Sent by `watch-feeds-cli` from `{}`.*\n",
                feed.url
            ));

            fs::write(&path, body)
                .with_context(|| format!("Failed to write event file at {}", path.display()))?;
            Ok(())
        })
    }
}

struct StdoutSink;

impl Sink for StdoutSink {
    fn write_deltas<'a>(
        &'a self,
        feed: &'a Feed,
        deltas: &'a [Entry],
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move {
            for d in deltas {
                let line = serde_json::json!({
                    "feed_id": feed.id,
                    "feed_url": feed.url,
                    "feed_title": feed.title,
                    "guid": d.guid,
                    "title": d.title,
                    "url": d.url,
                    "published_at": d.published_at.map(|t| t.to_rfc3339()),
                    "summary": d.summary,
                });
                println!("{line}");
            }
            Ok(())
        })
    }
}

struct NullSink;

impl Sink for NullSink {
    fn write_deltas<'a>(
        &'a self,
        _feed: &'a Feed,
        _deltas: &'a [Entry],
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move { Ok(()) })
    }
}
