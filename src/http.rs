use chrono::DateTime;
use serde::Deserialize;

use crate::Error;
use crate::Fetcher;

pub struct FeedFetcher {}

#[derive(Debug, Deserialize)]
struct Rss {
    channel: Channel,
}

#[derive(Debug, Deserialize)]
struct Channel {
    title: String,
    description: String,
    link: String,
    #[serde(rename = "item")]
    items: Vec<Item>,
}

#[derive(Debug, Deserialize)]
struct Item {
    title: String,
    link: String,
    guid: String,
    description: String,
    #[serde(rename = "pubDate")]
    pub_time: String,
}

impl Fetcher for FeedFetcher {
    async fn fetch(&self, url: &str) -> Result<(crate::RemoteFeed, Vec<crate::RemoteEntry>), Error> {
        let response = reqwest::get(url)
            .await
            .map_err(|err| Error::Internal(err.to_string()))?;

        // Handle the codes for better messaging to the user
        match response.status().into() {
            400..=499 => {
                return Err(Error::NotFound);
            }
            500..=599 => {
                return Err(Error::Internal(
                    "error received from the remote server".to_string(),
                ));
            }
            200..=299 => {} // Continue to parse and output
            _ => {}         // Continue to parse and output
        }

        let body = response
            .text()
            .await
            .map_err(|err| Error::Internal(err.to_string()))?;

        let rss: Rss =
            serde_xml_rs::from_str(&body).map_err(|err| Error::Internal(err.to_string()))?;

        // Parse the top level
        let feed = crate::RemoteFeed {
            url: rss.channel.link,
            title: rss.channel.title,
            description: rss.channel.description,
        };

        // Parse the entries
        let mut entries = vec![];
        for item in rss.channel.items {
            let publish_time_unix_secs = DateTime::parse_from_rfc2822(&item.pub_time)
                .ok()
                .and_then(|dt| u64::try_from(dt.timestamp()).ok());

            entries.push(crate::RemoteEntry {
                title: item.title,
                description: item.description,
                guid: item.guid,
                link: item.link,
                publish_time_unix_secs,
            });
        }

        Ok((feed, entries))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_RSS: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0">
<channel>
  <title>apenwarr</title>
  <description>apenwarr - NITLog</description>
  <link>https://apenwarr.ca/log/</link>
  <language>en-ca</language>
  <generator>PyNITLog</generator>
  <docs>http://blogs.law.harvard.edu/tech/rss</docs>
  <item>
    <title>Systems design 3: LLMs and the semantic revolution</title>
    <pubDate>Thu, 20 Nov 2025 14:19:14 +0000</pubDate>
    <link>https://apenwarr.ca/log/20251120</link>
    <guid isPermaLink="true">https://apenwarr.ca/log/20251120</guid>
    <description>&lt;p&gt;LLMs interconnect things. Anything. To anything.&lt;/p&gt;</description>
  </item>
  <item>
    <title>Billionaire math</title>
    <pubDate>Fri, 11 Jul 2025 12:00:00 +0000</pubDate>
    <link>https://apenwarr.ca/log/20250711</link>
    <guid isPermaLink="true">https://apenwarr.ca/log/20250711</guid>
    <description>&lt;p&gt;Software developers typically fall into the top 1-2% of earners globally.&lt;/p&gt;</description>
  </item>
</channel>
</rss>"#;

    #[tokio::test]
    async fn parses_rss() {
        let mut server = mockito::Server::new_async().await;
        let _mock = server
            .mock("GET", "/")
            .with_status(200)
            .with_header("content-type", "application/rss+xml")
            .with_body(SAMPLE_RSS)
            .create_async()
            .await;

        let (feed, entries) = FeedFetcher{}.fetch(&server.url()).await.unwrap();

        assert_eq!(feed.title, "apenwarr");
        assert_eq!(feed.description, "apenwarr - NITLog");
        assert_eq!(feed.url, "https://apenwarr.ca/log/");

        assert_eq!(entries.len(), 2);

        assert_eq!(
            entries[0].title,
            "Systems design 3: LLMs and the semantic revolution"
        );
        assert_eq!(entries[0].link, "https://apenwarr.ca/log/20251120");
        assert_eq!(entries[0].guid, "https://apenwarr.ca/log/20251120");
        assert_eq!(entries[0].publish_time_unix_secs, Some(1763648354));

        assert_eq!(entries[1].title, "Billionaire math");
        assert_eq!(entries[1].link, "https://apenwarr.ca/log/20250711");
        assert_eq!(entries[1].guid, "https://apenwarr.ca/log/20250711");
        assert_eq!(entries[1].publish_time_unix_secs, Some(1752235200));
    }

    #[tokio::test]
    async fn returns_not_found_on_4xx() {
        let mut server = mockito::Server::new_async().await;
        let _mock = server
            .mock("GET", "/")
            .with_status(404)
            .create_async()
            .await;

        let result = FeedFetcher{}.fetch(&server.url()).await;

        assert!(matches!(result, Err(Error::NotFound)));
    }

    #[tokio::test]
    async fn returns_internal_error_on_5xx() {
        let mut server = mockito::Server::new_async().await;
        let _mock = server
            .mock("GET", "/")
            .with_status(500)
            .create_async()
            .await;

        let result = FeedFetcher{}.fetch(&server.url()).await;

        assert!(matches!(result, Err(Error::Internal(_))));
    }
}
