/// This package provides the sqlite implementation of the seymour store.
///
/// It can be configured to point at a different database file, but most often
/// points at $HOME/.seymour/data.sqlite3.
use std::fs;
use std::path::PathBuf;

use rusqlite::Connection;

use crate::{Error, Feed, FeedEntry, RemoteEntry, RemoteFeed, Storage};

/// Store implementes all of the methods against a sqlite3 connection.
///
/// Constructing it runs all migrations so that obtaining one is ready to be used.
pub struct Store {
    conn: Connection,
}

impl Store {
    // Creates an instace of the storage that is backed by .seymour/data.sqlite3.
    pub fn new() -> Result<Self, Error> {
        let dir = dirs::home_dir()
            .ok_or_else(|| Error::Internal("could not determine home directory".into()))?
            .join(".seymour");

        fs::create_dir_all(&dir)?;

        let path: PathBuf = dir.join("data.sqlite3");
        let mut conn = Connection::open(&path)?;

        // Run migrations on connection
        MIGRATIONS
            .to_latest(&mut conn)
            .map_err(|err| Error::Internal(err.to_string()))?;

        Ok(Self { conn })
    }

    pub fn new_in_memory() -> Self {
        let mut conn =
            Connection::open_in_memory().expect("error opening in-memory sqlite connection");
        MIGRATIONS
            .to_latest(&mut conn)
            .expect("failed to run migrations");
        Self { conn }
    }
}

impl Storage for Store {
    async fn add_feed(&self, url: String) -> Result<Feed, Error> {
        let id = uuid::Uuid::new_v4().to_string();
        self.conn
            .execute("INSERT INTO feeds (id, url) VALUES (?1, ?2)", [&id, &url])?;

        self.conn.query_row(
            "SELECT id, url, title, description, last_synced_at, created_at, updated_at FROM feeds WHERE id = ?1",
            [&id],
            |row| {
                Ok(Feed {
                    id: row.get(0)?,
                    url: row.get(1)?,
                    title: row.get(2)?,
                    description: row.get(3)?,
                    last_synced_at: row.get::<_, Option<i64>>(4)?.map(|v| v as u64),
                    created_at: row.get::<_, i64>(5)? as u64,
                    updated_at: row.get::<_, i64>(6)? as u64,
                })
            },
        ).map_err(|err| err.into())
    }

    fn get_feed(&self, id: &str) -> Result<Feed, Error> {
        self.conn
            .query_row(
                "SELECT id, url, title, description, last_synced_at, created_at, updated_at FROM feeds WHERE id = ?1",
                [id],
                |row| {
                    Ok(Feed {
                        id: row.get(0)?,
                        url: row.get(1)?,
                        title: row.get(2)?,
                        description: row.get(3)?,
                        last_synced_at: row.get::<_, Option<i64>>(4)?.map(|v| v as u64),
                        created_at: row.get::<_, i64>(5)? as u64,
                        updated_at: row.get::<_, i64>(6)? as u64,
                    })
                },
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => Error::NotFound,
                other => other.into(),
            })
    }

    fn list_entries(&self, feed_id: &str) -> Result<Vec<FeedEntry>, Error> {
        let mut stmt = self.conn.prepare(
            "SELECT id, feed_id, title, description, guid, link, created_at, publish_time FROM feed_entries WHERE feed_id = ?1 ORDER BY publish_time DESC, created_at DESC"
        )?;
        let entry_iter = stmt.query_map([feed_id], |row| {
            Ok(FeedEntry {
                id: row.get(0)?,
                feed_id: row.get(1)?,
                title: row.get(2)?,
                description: row.get(3)?,
                guid: row.get(4)?,
                link: row.get(5)?,
                created_at: row.get::<_, i64>(6)? as u64,
                publish_time: row.get::<_, Option<i64>>(7)?.map(|v| v as u64),
            })
        })?;

        Ok(entry_iter.map(|e| e.unwrap()).collect())
    }

    fn update_feed(&self, feed_id: &str, remote: &RemoteFeed, entries: &[RemoteEntry]) -> Result<(), Error> {
        self.conn.execute(
            "UPDATE feeds SET title = ?1, description = ?2, last_synced_at = unixepoch() WHERE id = ?3",
            rusqlite::params![remote.title, remote.description, feed_id],
        )?;

        for entry in entries {
            let id = uuid::Uuid::new_v4().to_string();
            self.conn.execute(
                "INSERT OR IGNORE INTO feed_entries (id, feed_id, title, description, guid, link, publish_time) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                rusqlite::params![id, feed_id, entry.title, entry.description, entry.guid, entry.link, entry.publish_time_unix_secs.map(|s| s as i64)],
            )?;
        }

        Ok(())
    }

    /// Lists all feeds tracked within the store.
    fn list_feeds(&self) -> Result<Vec<Feed>, Error> {
        let mut stmt = self.conn.prepare(
            "SELECT id, url, title, description, last_synced_at, created_at, updated_at FROM feeds;"
        )?;
        let fd_iter = stmt.query_map([], |row| {
            Ok(Feed {
                id: row.get(0)?,
                url: row.get(1)?,
                title: row.get(2)?,
                description: row.get(3)?,
                last_synced_at: row.get::<_, Option<i64>>(4)?.map(|v| v as u64),
                created_at: row.get::<_, i64>(5)? as u64,
                updated_at: row.get::<_, i64>(6)? as u64,
            })
        })?;

        Ok(fd_iter.map(|fd| fd.unwrap()).collect())
    }
}

use rusqlite_migration::{Migrations, M};

const MIGRATIONS_SLICE: &[M<'_>] = &[
    M::up(
        "CREATE TABLE feeds (
            id TEXT PRIMARY KEY,
            url TEXT NOT NULL UNIQUE,
            title TEXT,
            description TEXT,
            last_synced_at INTEGER,
            created_at INTEGER NOT NULL DEFAULT (unixepoch()),
            updated_at INTEGER NOT NULL DEFAULT (unixepoch())
        );",
    ),
    M::up(
        "CREATE TABLE feed_entries (
            id TEXT PRIMARY KEY,
            feed_id TEXT NOT NULL,
            title TEXT NOT NULL,
            description TEXT NOT NULL,
            guid TEXT NOT NULL UNIQUE,
            created_at INTEGER NOT NULL DEFAULT (unixepoch()),
            publish_time INTEGER NULL,
            link VARCHAR(256) NOT NULL
        );",
    ),
];
const MIGRATIONS: Migrations<'_> = Migrations::from_slice(MIGRATIONS_SLICE);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_feeds_returns_empty_list() {
        let store = Store::new_in_memory();
        let feeds = store.list_feeds().unwrap();
        assert!(feeds.is_empty());
    }

    #[test]
    fn get_feed_returns_not_found() {
        let store = Store::new_in_memory();
        let result = store.get_feed("nonexistent-id");
        assert!(matches!(result, Err(Error::NotFound)));
    }

    #[tokio::test]
    async fn get_feed_returns_inserted_feed() {
        let store = Store::new_in_memory();
        let added = store
            .add_feed("https://example.com/rss".into())
            .await
            .unwrap();
        let fetched = store.get_feed(&added.id).unwrap();
        assert_eq!(fetched.id, added.id);
        assert_eq!(fetched.url, "https://example.com/rss");
    }

    #[test]
    fn list_entries_returns_empty_for_unknown_feed() {
        let store = Store::new_in_memory();
        let entries = store.list_entries("nonexistent-feed-id").unwrap();
        assert!(entries.is_empty());
    }

    #[tokio::test]
    async fn list_entries_returns_inserted_entries() {
        let store = Store::new_in_memory();
        let feed = store
            .add_feed("https://example.com/rss".into())
            .await
            .unwrap();
        store.conn.execute(
            "INSERT INTO feed_entries (id, feed_id, title, description, guid, link, publish_time) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                "entry-1", &feed.id, "First Post", "Description 1", "guid-1", "https://example.com/1", 1767312000i64 // 2026-01-02 00:00:00 UTC
            ],
        ).unwrap();
        store.conn.execute(
            "INSERT INTO feed_entries (id, feed_id, title, description, guid, link, publish_time) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                "entry-2", &feed.id, "Second Post", "Description 2", "guid-2", "https://example.com/2", 1767398400i64 // 2026-01-03 00:00:00 UTC
            ],
        ).unwrap();

        let entries = store.list_entries(&feed.id).unwrap();
        assert_eq!(entries.len(), 2);
        // Ordered by publish_time DESC
        assert_eq!(entries[0].title, "Second Post");
        assert_eq!(entries[1].title, "First Post");
    }
}
