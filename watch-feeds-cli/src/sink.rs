use anyhow::Result;

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

/// BorAI inbox sink. Writes one markdown event file per delta to
/// $BORAI_INBOX_PATH/events/ and updates _index.json. NOT IMPLEMENTED YET —
/// pending the design call on the `source_update` event type addition to
/// the BorAI staging schema.
struct BoraiInboxSink;

impl Sink for BoraiInboxSink {
    fn write_deltas<'a>(
        &'a self,
        _feed: &'a Feed,
        _deltas: &'a [Entry],
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move {
            anyhow::bail!(
                "BorAI inbox sink not yet implemented — pending source_update event type design. \
                 Use --sink stdout to verify scan output for now."
            )
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
