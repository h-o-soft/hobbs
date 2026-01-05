//! RSS reader module for HOBBS.
//!
//! This module provides RSS feed subscription and reading functionality.

pub mod fetcher;
pub mod repository;
pub mod service;
pub mod types;
pub mod updater;

pub use fetcher::{fetch_feed, validate_url, RssFetcher};
pub use repository::{RssFeedRepository, RssItemRepository, RssReadPositionRepository};
pub use service::{AddFeedRequest, RssService};
pub use types::{
    NewRssFeed, NewRssItem, ParsedFeed, ParsedItem, RssFeed, RssFeedUpdate, RssFeedWithUnread,
    RssItem, RssReadPosition, DEFAULT_FETCH_INTERVAL, MAX_CONSECUTIVE_ERRORS,
    MAX_DESCRIPTION_LENGTH, MAX_FEED_SIZE, MAX_ITEMS_PER_FEED,
};
pub use updater::{
    start_rss_updater, start_rss_updater_with_config, start_rss_updater_with_interval, RssUpdater,
    DEFAULT_CHECK_INTERVAL_SECS,
};
