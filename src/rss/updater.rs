//! RSS background updater for HOBBS.
//!
//! This module provides background task for periodically updating RSS feeds.

use std::sync::Arc;

use tokio::time::{interval, Duration};
use tracing::{debug, error, info, warn};

use crate::db::Database;
use crate::rss::fetcher::fetch_feed;
use crate::rss::repository::{RssFeedRepository, RssItemRepository};
use crate::rss::types::{NewRssItem, MAX_CONSECUTIVE_ERRORS, MAX_ITEMS_PER_FEED};

/// Default update check interval in seconds (5 minutes).
pub const DEFAULT_CHECK_INTERVAL_SECS: u64 = 300;

/// RSS feed background updater.
///
/// This struct manages a background task that periodically checks for
/// feeds that need updating and fetches new items.
pub struct RssUpdater {
    db: Arc<Database>,
    check_interval: Duration,
}

impl RssUpdater {
    /// Create a new RssUpdater with the given database.
    pub fn new(db: Arc<Database>) -> Self {
        Self {
            db,
            check_interval: Duration::from_secs(DEFAULT_CHECK_INTERVAL_SECS),
        }
    }

    /// Create a new RssUpdater with a custom check interval.
    pub fn with_interval(db: Arc<Database>, interval_secs: u64) -> Self {
        Self {
            db,
            check_interval: Duration::from_secs(interval_secs),
        }
    }

    /// Run the updater loop.
    ///
    /// This method runs indefinitely, checking for feeds that need
    /// updating at the configured interval.
    pub async fn run(&self) {
        info!(
            "RSS updater started (check interval: {} seconds)",
            self.check_interval.as_secs()
        );

        let mut timer = interval(self.check_interval);

        loop {
            timer.tick().await;
            self.update_due_feeds().await;
        }
    }

    /// Check and update all feeds that are due for update.
    async fn update_due_feeds(&self) {
        debug!("Checking for feeds due for update");

        // Get feeds that need updating
        let feeds = match RssFeedRepository::list_due_for_fetch(self.db.conn()) {
            Ok(feeds) => feeds,
            Err(e) => {
                error!("Failed to list feeds due for update: {}", e);
                return;
            }
        };

        if feeds.is_empty() {
            debug!("No feeds due for update");
            return;
        }

        info!("Updating {} feed(s)", feeds.len());

        for feed in feeds {
            self.update_feed(feed.id, &feed.url).await;
        }
    }

    /// Update a single feed.
    async fn update_feed(&self, feed_id: i64, url: &str) {
        debug!("Updating feed {}: {}", feed_id, url);

        match fetch_feed(url).await {
            Ok(parsed) => {
                let mut new_count = 0;

                // Store items
                for item in parsed.items.into_iter().take(MAX_ITEMS_PER_FEED) {
                    let mut new_item = NewRssItem::new(feed_id, &item.guid, &item.title);
                    if let Some(link) = item.link {
                        new_item = new_item.with_link(link);
                    }
                    if let Some(desc) = item.description {
                        new_item = new_item.with_description(desc);
                    }
                    if let Some(author) = item.author {
                        new_item = new_item.with_author(author);
                    }
                    if let Some(published_at) = item.published_at {
                        new_item = new_item.with_published_at(published_at);
                    }

                    match RssItemRepository::create_or_ignore(self.db.conn(), &new_item) {
                        Ok(Some(_)) => new_count += 1,
                        Ok(None) => {} // Already exists
                        Err(e) => {
                            error!("Failed to store item for feed {}: {}", feed_id, e);
                        }
                    }
                }

                // Clear error and update last_fetched
                if let Err(e) = RssFeedRepository::clear_error(self.db.conn(), feed_id) {
                    error!("Failed to clear error for feed {}: {}", feed_id, e);
                }

                // Prune old items
                if let Err(e) = RssItemRepository::prune_old_items(self.db.conn(), feed_id) {
                    error!("Failed to prune old items for feed {}: {}", feed_id, e);
                }

                if new_count > 0 {
                    info!("Feed {} updated: {} new item(s)", feed_id, new_count);
                } else {
                    debug!("Feed {} updated: no new items", feed_id);
                }
            }
            Err(e) => {
                warn!("Failed to fetch feed {}: {}", feed_id, e);

                // Increment error count
                if let Err(err) =
                    RssFeedRepository::increment_error(self.db.conn(), feed_id, &e.to_string())
                {
                    error!("Failed to increment error for feed {}: {}", feed_id, err);
                }

                // Check if feed should be disabled
                match RssFeedRepository::get_by_id(self.db.conn(), feed_id) {
                    Ok(Some(feed)) => {
                        if feed.error_count >= MAX_CONSECUTIVE_ERRORS {
                            warn!(
                                "Feed {} disabled after {} consecutive errors",
                                feed_id, MAX_CONSECUTIVE_ERRORS
                            );
                        }
                    }
                    Ok(None) => {} // Feed was deleted
                    Err(e) => {
                        error!("Failed to get feed {}: {}", feed_id, e);
                    }
                }
            }
        }
    }
}

/// Start the RSS updater as a background task.
///
/// This function spawns the updater on the current LocalSet.
/// Call this from within a LocalSet context (e.g., in main.rs).
pub fn start_rss_updater(db: Arc<Database>) {
    let updater = RssUpdater::new(db);
    tokio::task::spawn_local(async move {
        updater.run().await;
    });
}

/// Start the RSS updater with a custom check interval.
pub fn start_rss_updater_with_interval(db: Arc<Database>, interval_secs: u64) {
    let updater = RssUpdater::with_interval(db, interval_secs);
    tokio::task::spawn_local(async move {
        updater.run().await;
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rss_updater_new() {
        let db = Arc::new(Database::open_in_memory().unwrap());
        let updater = RssUpdater::new(db);
        assert_eq!(
            updater.check_interval,
            Duration::from_secs(DEFAULT_CHECK_INTERVAL_SECS)
        );
    }

    #[test]
    fn test_rss_updater_with_interval() {
        let db = Arc::new(Database::open_in_memory().unwrap());
        let updater = RssUpdater::with_interval(db, 60);
        assert_eq!(updater.check_interval, Duration::from_secs(60));
    }
}
