use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone)]
pub struct Feed {
    pub id: i64,
    pub url: String,
    pub title: Option<String>,
    pub cadence_minutes: i64,
    pub last_scanned_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct Entry {
    pub guid: String,
    pub title: String,
    pub url: String,
    pub published_at: Option<DateTime<Utc>>,
    pub summary: Option<String>,
}

/// Open a SQLite connection, applying the schema migration on first run.
/// Path resolution mirrors ask-grok-cli's Option B pattern: project
/// `.claude/feeds.db` if a `.claude/` already exists at git root, else the
/// global `~/.local/share/watch-feeds-cli/feeds.db`. Override via --state-db.
pub fn open(override_path: Option<&str>) -> Result<Connection> {
    let path = match override_path {
        Some(p) => PathBuf::from(p),
        None => resolve_state_db_path()?,
    };
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent).with_context(|| {
                format!("Failed to create state directory at {}", parent.display())
            })?;
        }
    }
    let conn = Connection::open(&path)
        .with_context(|| format!("Failed to open SQLite at {}", path.display()))?;
    apply_schema(&conn)?;
    Ok(conn)
}

fn resolve_state_db_path() -> Result<PathBuf> {
    if let Some(project_root) = resolve_project_root_if_present() {
        let project_claude = project_root.join(".claude");
        if project_claude.is_dir() {
            return Ok(project_claude.join("feeds.db"));
        }
    }
    let home = env::var("HOME").context("HOME env var not set")?;
    Ok(PathBuf::from(home)
        .join(".local")
        .join("share")
        .join("watch-feeds-cli")
        .join("feeds.db"))
}

fn resolve_project_root_if_present() -> Option<PathBuf> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if stdout.is_empty() {
        None
    } else {
        Some(PathBuf::from(stdout))
    }
}

fn apply_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS feeds (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            url TEXT NOT NULL UNIQUE,
            title TEXT,
            cadence_minutes INTEGER NOT NULL DEFAULT 20160,
            last_scanned_at TEXT
        );

        CREATE TABLE IF NOT EXISTS entries (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            feed_id INTEGER NOT NULL REFERENCES feeds(id) ON DELETE CASCADE,
            guid TEXT NOT NULL,
            title TEXT NOT NULL,
            url TEXT NOT NULL,
            published_at TEXT,
            seen_at TEXT NOT NULL,
            UNIQUE(feed_id, guid)
        );

        CREATE INDEX IF NOT EXISTS idx_entries_feed ON entries(feed_id);
        ",
    )
    .context("schema migration failed")?;
    Ok(())
}

pub fn add_feed(
    conn: &Connection,
    url: &str,
    title: Option<&str>,
    cadence_minutes: i64,
) -> Result<i64> {
    conn.execute(
        "INSERT OR IGNORE INTO feeds (url, title, cadence_minutes) VALUES (?1, ?2, ?3)",
        params![url, title, cadence_minutes],
    )?;
    let id: i64 = conn.query_row(
        "SELECT id FROM feeds WHERE url = ?1",
        params![url],
        |row| row.get(0),
    )?;
    Ok(id)
}

pub fn list_feeds(conn: &Connection) -> Result<Vec<Feed>> {
    let mut stmt = conn.prepare(
        "SELECT id, url, title, cadence_minutes, last_scanned_at FROM feeds ORDER BY id",
    )?;
    let rows = stmt.query_map([], |row| {
        let last_scanned: Option<String> = row.get(4)?;
        Ok(Feed {
            id: row.get(0)?,
            url: row.get(1)?,
            title: row.get(2)?,
            cadence_minutes: row.get(3)?,
            last_scanned_at: last_scanned
                .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                .map(|d| d.with_timezone(&Utc)),
        })
    })?;
    let mut feeds = Vec::new();
    for r in rows {
        feeds.push(r?);
    }
    Ok(feeds)
}

pub fn remove_feed(conn: &Connection, target: &str) -> Result<bool> {
    let rows = if let Ok(id) = target.parse::<i64>() {
        conn.execute("DELETE FROM feeds WHERE id = ?1", params![id])?
    } else {
        conn.execute("DELETE FROM feeds WHERE url = ?1", params![target])?
    };
    Ok(rows > 0)
}

pub fn touch_last_scanned(conn: &Connection, feed_id: i64) -> Result<()> {
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE feeds SET last_scanned_at = ?1 WHERE id = ?2",
        params![now, feed_id],
    )?;
    Ok(())
}

pub fn record_entries(conn: &Connection, feed_id: i64, entries: &[Entry]) -> Result<()> {
    let now = Utc::now().to_rfc3339();
    for e in entries {
        let published = e.published_at.map(|d| d.to_rfc3339());
        conn.execute(
            "INSERT OR IGNORE INTO entries (feed_id, guid, title, url, published_at, seen_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![feed_id, e.guid, e.title, e.url, published, now],
        )?;
    }
    Ok(())
}

pub fn entry_already_seen(conn: &Connection, feed_id: i64, guid: &str) -> Result<bool> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM entries WHERE feed_id = ?1 AND guid = ?2",
        params![feed_id, guid],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

#[allow(dead_code)]
pub fn state_path_for_test(home: &Path) -> PathBuf {
    home.join(".local")
        .join("share")
        .join("watch-feeds-cli")
        .join("feeds.db")
}
