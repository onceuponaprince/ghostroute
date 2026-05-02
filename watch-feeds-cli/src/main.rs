use anyhow::Result;
use clap::{Parser, Subcommand};

mod cli;
mod db;
mod feed;
mod sink;

#[derive(Parser, Debug)]
#[command(
    name = "watch-feeds-cli",
    about = "Scan RSS / Atom / JSON Feeds, diff against last-seen state, post deltas.",
    version
)]
struct Args {
    #[command(subcommand)]
    command: Command,

    /// Override SQLite state path. Default resolves via the same project-then-
    /// global pattern as ask-grok-cli: project `.claude/feeds.db` if a
    /// `.claude/` already exists at git root, else `~/.local/share/watch-feeds-cli/feeds.db`.
    #[arg(long, global = true)]
    state_db: Option<String>,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Fetch every configured feed once, post deltas, exit. External scheduler
    /// (cron / systemd timer / `/schedule`) drives cadence.
    Scan {
        /// Override the inbox sink target. Without this flag, deltas write to
        /// $BORAI_INBOX_PATH/events/ as markdown event files.
        #[arg(long, value_enum, default_value_t = cli::SinkTarget::BoraiInbox)]
        sink: cli::SinkTarget,
    },
    /// Long-running daemon. Sleeps until next due feed (per-feed cadence),
    /// fetches, posts deltas, repeats. SIGTERM / Ctrl-C exit cleanly.
    Watch {
        #[arg(long, value_enum, default_value_t = cli::SinkTarget::BoraiInbox)]
        sink: cli::SinkTarget,
    },
    /// Add a feed to the watch list.
    AddFeed {
        /// Feed URL (RSS, Atom, or JSON Feed).
        url: String,
        /// Cadence in minutes. 1440 = daily, 10080 = weekly, 20160 = fortnightly.
        #[arg(long, default_value_t = 20160)]
        cadence_minutes: i64,
        /// Optional human label. Defaults to the feed's <title> on first fetch.
        #[arg(long)]
        title: Option<String>,
    },
    /// Print the current watch list with last-scanned timestamps.
    ListFeeds,
    /// Remove a feed by URL or by id.
    RemoveFeed {
        /// Either the feed URL or the numeric id from `list-feeds`.
        target: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    match args.command {
        Command::Scan { sink } => cli::run_scan(args.state_db.as_deref(), sink).await,
        Command::Watch { sink } => cli::run_watch(args.state_db.as_deref(), sink).await,
        Command::AddFeed {
            url,
            cadence_minutes,
            title,
        } => cli::run_add_feed(args.state_db.as_deref(), &url, cadence_minutes, title.as_deref()),
        Command::ListFeeds => cli::run_list_feeds(args.state_db.as_deref()),
        Command::RemoveFeed { target } => cli::run_remove_feed(args.state_db.as_deref(), &target),
    }
}
