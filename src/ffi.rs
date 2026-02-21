use std::sync::Arc;

use crate::{http::FeedFetcher, sqlite::Store, Core, Error, Feed, FeedEntry};

/// FFICore is the concrete entry point for FFI consumers (e.g. Swift via UniFFI).
/// It wraps Core with fixed concrete types so the FFI layer sees no generics.
pub struct FFICore(Core<Store, FeedFetcher>);

impl FFICore {
    pub fn new() -> Result<Arc<Self>, Error> {
        let store = Store::new()?;
        let core = Core::new(store, FeedFetcher {});
        Ok(Arc::new(Self(core)))
    }

    pub fn list_feeds(&self) -> Result<Vec<Feed>, Error> {
        self.0.list_feeds()
    }

    pub async fn add_feed(&self, url: String) -> Result<Feed, Error> {
        self.0.add_feed(url).await
    }

    pub fn get_feed(&self, id: &str) -> Result<Feed, Error> {
        self.0.get_feed(id)
    }

    pub fn list_entries(&self, feed_id: &str) -> Result<Vec<FeedEntry>, Error> {
        self.0.list_entries(feed_id)
    }
}
