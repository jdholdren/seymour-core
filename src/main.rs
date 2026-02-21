use std::io::{self, Write};

use clap::{Parser, Subcommand};
use seycore::{sqlite::Store, Storage};

#[derive(Parser)]
#[command(name = "seymour")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List all feeds, or describe one feed by ID
    Feeds {
        /// Feed ID to describe; omit to list all feeds
        id: Option<String>,
    },
    /// Add a feed
    Add { url: String },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let store = Store::new()?;

    match cli.command {
        Commands::Feeds { id: Some(id) } => handle_describe_feed(&store, &id, io::stdout())?,
        Commands::Feeds { id: None } => handle_list_feeds(&store, io::stdout())?,
        Commands::Add { url } => handle_add_feed(&store, url, io::stdout())?,
    }

    Ok(())
}

/// Prints all fields for a single feed in a right-aligned key-value layout:
///
/// ```text
///           ID: 550e8400-e29b-41d4-a716-446655440000
///          URL: https://example.com/rss
///        Title: My Blog
///  Description: A blog about things
///  Last Synced: 2026-02-16 12:00:00
///      Created: 2026-02-15 08:30:00
///      Updated: 2026-02-16 12:00:00
/// ```
fn handle_describe_feed(store: &impl Storage, id: &str, mut out: impl Write) -> anyhow::Result<()> {
    let feed = store.get_feed(id)?;
    let none = "—".to_string();
    writeln!(out, "{:>12}: {}", "ID", feed.id)?;
    writeln!(out, "{:>12}: {}", "URL", feed.url)?;
    writeln!(
        out,
        "{:>12}: {}",
        "Title",
        feed.title.as_deref().unwrap_or(&none)
    )?;
    writeln!(
        out,
        "{:>12}: {}",
        "Description",
        feed.description.as_deref().unwrap_or(&none)
    )?;
    writeln!(
        out,
        "{:>12}: {}",
        "Last Synced",
        feed.last_synced_at.as_deref().unwrap_or(&none)
    )?;
    writeln!(out, "{:>12}: {}", "Created", feed.created_at)?;
    writeln!(out, "{:>12}: {}", "Updated", feed.updated_at)?;
    Ok(())
}

fn handle_add_feed(store: &impl Storage, url: String, mut out: impl Write) -> anyhow::Result<()> {
    let feed = store.add_feed(url)?;
    writeln!(out, "added feed {} ({})", feed.id, feed.url)?;
    Ok(())
}

fn handle_list_feeds(store: &impl Storage, mut out: impl Write) -> anyhow::Result<()> {
    let feeds = store.list_feeds()?;
    let rows: Vec<Vec<String>> = feeds
        .iter()
        .map(|f| vec![f.id.clone(), f.url.clone()])
        .collect();
    write_table(&["ID", "URL"], &rows, &mut out)?;
    Ok(())
}

const MAX_COL_WIDTH: usize = 36;

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max - 3])
    }
}

fn write_table(headers: &[&str], rows: &[Vec<String>], mut out: impl Write) -> io::Result<()> {
    let mut col_widths: Vec<usize> = headers.iter().map(|h| h.len()).collect();
    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            if cell.len() > col_widths[i] {
                col_widths[i] = cell.len();
            }
        }
    }
    for w in col_widths.iter_mut() {
        *w = (*w).min(MAX_COL_WIDTH);
    }

    let last = col_widths.len() - 1;

    // Header row
    let header_parts: Vec<String> = headers
        .iter()
        .enumerate()
        .map(|(i, h)| {
            let val = truncate(h, col_widths[i]);
            if i == last {
                val
            } else {
                format!("{:<width$}", val, width = col_widths[i])
            }
        })
        .collect();
    writeln!(out, "{}", header_parts.join("  "))?;

    // Separator
    let sep_parts: Vec<String> = col_widths.iter().map(|&w| "-".repeat(w)).collect();
    writeln!(out, "{}", sep_parts.join("  "))?;

    // Data rows
    for row in rows {
        let parts: Vec<String> = row
            .iter()
            .enumerate()
            .map(|(i, cell)| {
                let val = truncate(cell, col_widths[i]);
                if i == last {
                    val
                } else {
                    format!("{:<width$}", val, width = col_widths[i])
                }
            })
            .collect();
        writeln!(out, "{}", parts.join("  "))?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use seycore::{Error, Feed};
    use std::path::PathBuf;

    struct MockStore {
        feeds: Vec<Feed>,
    }

    impl Default for MockStore {
        fn default() -> Self {
            Self {
                feeds: vec![
                    Feed {
                        id: "00000000-0000-0000-0000-000000000001".into(),
                        url: "https://example.com/rss".into(),
                        title: Some("Example Blog".into()),
                        description: Some("A blog about things".into()),
                        last_synced_at: None,
                        created_at: "2026-01-01 00:00:00".into(),
                        updated_at: "2026-01-01 00:00:00".into(),
                    },
                    Feed {
                        id: "00000000-0000-0000-0000-000000000002".into(),
                        url: "https://example.com/atom".into(),
                        title: Some("Another Blog".into()),
                        description: None,
                        last_synced_at: None,
                        created_at: "2026-01-02 00:00:00".into(),
                        updated_at: "2026-01-02 00:00:00".into(),
                    },
                ],
            }
        }
    }

    impl Storage for MockStore {
        fn list_feeds(&self) -> Result<Vec<Feed>, Error> {
            Ok(self.feeds.clone())
        }

        fn add_feed(&self, url: String) -> Result<Feed, Error> {
            Ok(Feed {
                url,
                ..self.feeds.first().unwrap().clone()
            })
        }

        fn get_feed(&self, id: &str) -> Result<Feed, Error> {
            match id {
                "00000000-0000-0000-0000-000000000001" => Ok(self.feeds.first().unwrap().clone()),
                _ => Err(Error::NotFound),
            }
        }
    }

    fn golden(name: &str) -> String {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("testdata")
            .join(name);
        std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("failed to read golden file {}: {e}", path.display()))
    }

    #[test]
    fn add_feed_output() {
        let mut buf = Vec::new();
        handle_add_feed(
            &MockStore::default(),
            "https://example.com/rss".into(),
            &mut buf,
        )
        .unwrap();
        let output = String::from_utf8(buf).unwrap();
        assert_eq!(output, golden("add_feed.txt"));
    }

    #[test]
    fn describe_feed_output() {
        let mut buf = Vec::new();
        handle_describe_feed(
            &MockStore::default(),
            "00000000-0000-0000-0000-000000000001",
            &mut buf,
        )
        .unwrap();
        let output = String::from_utf8(buf).unwrap();
        assert_eq!(output, golden("describe_feed.txt"));
    }

    #[test]
    fn list_feeds_output() {
        let mut buf = Vec::new();
        handle_list_feeds(&MockStore::default(), &mut buf).unwrap();
        let output = String::from_utf8(buf).unwrap();
        assert_eq!(output, golden("list_feeds.txt"));
    }
}
