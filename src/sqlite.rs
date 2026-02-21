/// This package provides the sqlite implementation of the seymour store.
///
/// It can be configured to point at a different database file, but most often
/// points at $HOME/.seymour/data.sqlite3.
use std::fs;
use std::path::PathBuf;

use rusqlite::Connection;

use crate::{Error, Feed, Storage};

/// Store implementes all of the methods against a sqlite3 connection.
///
/// Constructing it runs all migrations so that obtaining one is ready to be used.
pub struct Store {
    conn: Connection,
}

impl Default for Store {
    /// Creates an implementation of Store that talks to an in-memory sqlite.
    fn default() -> Self {
        let mut conn =
            Connection::open_in_memory().expect("error opening in-memory sqlite connection");
        MIGRATIONS
            .to_latest(&mut conn)
            .expect("failed to run migrations");
        Self { conn }
    }
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
}

impl Storage for Store {
    fn add_feed(&self, url: String) -> Result<Feed, Error> {
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
                    last_synced_at: row.get(4)?,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
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
                        last_synced_at: row.get(4)?,
                        created_at: row.get(5)?,
                        updated_at: row.get(6)?,
                    })
                },
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => Error::NotFound,
                other => other.into(),
            })
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
                last_synced_at: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
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
            last_synced_at DATETIME,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        );",
    ),
    M::up(
        "CREATE TABLE feed_entries (
            id TEXT PRIMARY KEY,
            feed_id TEXT NOT NULL,
            title TEXT NOT NULL,
            description TEXT NOT NULL,
            guid TEXT NOT NULL UNIQUE,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            publish_time DATETIME NULL,
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
        let store = Store::default();
        let feeds = store.list_feeds().unwrap();
        assert!(feeds.is_empty());
    }

    #[test]
    fn get_feed_returns_not_found() {
        let store = Store::default();
        let result = store.get_feed("nonexistent-id");
        assert!(matches!(result, Err(Error::NotFound)));
    }

    #[test]
    fn get_feed_returns_inserted_feed() {
        let store = Store::default();
        let added = store.add_feed("https://example.com/rss".into()).unwrap();
        let fetched = store.get_feed(&added.id).unwrap();
        assert_eq!(fetched.id, added.id);
        assert_eq!(fetched.url, "https://example.com/rss");
    }
}
