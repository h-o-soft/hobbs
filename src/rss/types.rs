//! RSS types for HOBBS.

use chrono::{DateTime, Utc};

/// Maximum length for RSS item description.
pub const MAX_DESCRIPTION_LENGTH: usize = 10000;

/// Maximum number of items to store per feed.
pub const MAX_ITEMS_PER_FEED: usize = 100;

/// Maximum feed size in bytes (5MB).
pub const MAX_FEED_SIZE: u64 = 5 * 1024 * 1024;

/// Default fetch interval in seconds (1 hour).
pub const DEFAULT_FETCH_INTERVAL: i64 = 3600;

/// Maximum consecutive errors before disabling a feed.
pub const MAX_CONSECUTIVE_ERRORS: i32 = 5;

/// An RSS feed.
#[derive(Debug, Clone)]
pub struct RssFeed {
    /// Feed ID.
    pub id: i64,
    /// Feed URL.
    pub url: String,
    /// Feed title.
    pub title: String,
    /// Feed description.
    pub description: Option<String>,
    /// Site URL (the website the feed belongs to).
    pub site_url: Option<String>,
    /// Last time the feed was fetched.
    pub last_fetched_at: Option<DateTime<Utc>>,
    /// Timestamp of the newest item.
    pub last_item_at: Option<DateTime<Utc>>,
    /// Fetch interval in seconds.
    pub fetch_interval: i64,
    /// Whether the feed is active.
    pub is_active: bool,
    /// Number of consecutive fetch errors.
    pub error_count: i32,
    /// Last error message.
    pub last_error: Option<String>,
    /// User ID who created the feed.
    pub created_by: i64,
    /// When the feed was created.
    pub created_at: DateTime<Utc>,
    /// When the feed was last updated.
    pub updated_at: DateTime<Utc>,
}

impl RssFeed {
    /// Check if the feed should be fetched based on the interval.
    pub fn is_due_for_fetch(&self) -> bool {
        if !self.is_active {
            return false;
        }
        match self.last_fetched_at {
            None => true,
            Some(last) => {
                let elapsed = Utc::now().signed_duration_since(last);
                elapsed.num_seconds() >= self.fetch_interval
            }
        }
    }

    /// Check if the feed has exceeded the error threshold.
    pub fn has_exceeded_error_threshold(&self) -> bool {
        self.error_count >= MAX_CONSECUTIVE_ERRORS
    }
}

/// New RSS feed for creation.
#[derive(Debug, Clone)]
pub struct NewRssFeed {
    /// Feed URL.
    pub url: String,
    /// Feed title.
    pub title: String,
    /// Feed description.
    pub description: Option<String>,
    /// Site URL.
    pub site_url: Option<String>,
    /// User ID who created the feed.
    pub created_by: i64,
}

impl NewRssFeed {
    /// Create a new feed.
    pub fn new(url: impl Into<String>, title: impl Into<String>, created_by: i64) -> Self {
        Self {
            url: url.into(),
            title: title.into(),
            description: None,
            site_url: None,
            created_by,
        }
    }

    /// Set the description.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the site URL.
    pub fn with_site_url(mut self, site_url: impl Into<String>) -> Self {
        self.site_url = Some(site_url.into());
        self
    }
}

/// RSS feed update request.
#[derive(Debug, Clone, Default)]
pub struct RssFeedUpdate {
    /// New title.
    pub title: Option<String>,
    /// New description.
    pub description: Option<Option<String>>,
    /// New fetch interval.
    pub fetch_interval: Option<i64>,
    /// Whether the feed is active.
    pub is_active: Option<bool>,
}

impl RssFeedUpdate {
    /// Create a new update request.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the title.
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the fetch interval.
    pub fn with_fetch_interval(mut self, interval: i64) -> Self {
        self.fetch_interval = Some(interval);
        self
    }

    /// Enable the feed.
    pub fn enable(mut self) -> Self {
        self.is_active = Some(true);
        self
    }

    /// Disable the feed.
    pub fn disable(mut self) -> Self {
        self.is_active = Some(false);
        self
    }

    /// Check if the update is empty.
    pub fn is_empty(&self) -> bool {
        self.title.is_none()
            && self.description.is_none()
            && self.fetch_interval.is_none()
            && self.is_active.is_none()
    }
}

/// An RSS item (article).
#[derive(Debug, Clone)]
pub struct RssItem {
    /// Item ID.
    pub id: i64,
    /// Feed ID this item belongs to.
    pub feed_id: i64,
    /// Unique identifier for the item (RSS guid or Atom id).
    pub guid: String,
    /// Item title.
    pub title: String,
    /// Link to the original article.
    pub link: Option<String>,
    /// Item description/summary (HTML tags stripped).
    pub description: Option<String>,
    /// Author name.
    pub author: Option<String>,
    /// When the item was published.
    pub published_at: Option<DateTime<Utc>>,
    /// When the item was fetched.
    pub fetched_at: DateTime<Utc>,
}

/// New RSS item for creation.
#[derive(Debug, Clone)]
pub struct NewRssItem {
    /// Feed ID.
    pub feed_id: i64,
    /// Unique identifier.
    pub guid: String,
    /// Item title.
    pub title: String,
    /// Link to the original article.
    pub link: Option<String>,
    /// Item description.
    pub description: Option<String>,
    /// Author name.
    pub author: Option<String>,
    /// When the item was published.
    pub published_at: Option<DateTime<Utc>>,
}

impl NewRssItem {
    /// Create a new item.
    pub fn new(feed_id: i64, guid: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            feed_id,
            guid: guid.into(),
            title: title.into(),
            link: None,
            description: None,
            author: None,
            published_at: None,
        }
    }

    /// Set the link.
    pub fn with_link(mut self, link: impl Into<String>) -> Self {
        self.link = Some(link.into());
        self
    }

    /// Set the description.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        let desc = description.into();
        // Truncate if too long
        if desc.len() > MAX_DESCRIPTION_LENGTH {
            self.description = Some(desc.chars().take(MAX_DESCRIPTION_LENGTH).collect());
        } else {
            self.description = Some(desc);
        }
        self
    }

    /// Set the author.
    pub fn with_author(mut self, author: impl Into<String>) -> Self {
        self.author = Some(author.into());
        self
    }

    /// Set the published date.
    pub fn with_published_at(mut self, published_at: DateTime<Utc>) -> Self {
        self.published_at = Some(published_at);
        self
    }
}

/// RSS read position for tracking user's last read item.
#[derive(Debug, Clone)]
pub struct RssReadPosition {
    /// Read position ID.
    pub id: i64,
    /// User ID.
    pub user_id: i64,
    /// Feed ID.
    pub feed_id: i64,
    /// Last read item ID.
    pub last_read_item_id: Option<i64>,
    /// When the position was last updated.
    pub last_read_at: DateTime<Utc>,
}

/// Parsed feed data from external source.
#[derive(Debug, Clone)]
pub struct ParsedFeed {
    /// Feed title.
    pub title: String,
    /// Feed description.
    pub description: Option<String>,
    /// Site URL.
    pub site_url: Option<String>,
    /// Parsed items.
    pub items: Vec<ParsedItem>,
}

/// Parsed item data from external source.
#[derive(Debug, Clone)]
pub struct ParsedItem {
    /// Unique identifier.
    pub guid: String,
    /// Item title.
    pub title: String,
    /// Link to the original article.
    pub link: Option<String>,
    /// Item description (HTML tags stripped).
    pub description: Option<String>,
    /// Author name.
    pub author: Option<String>,
    /// When the item was published.
    pub published_at: Option<DateTime<Utc>>,
}

/// Feed with unread count for display.
#[derive(Debug, Clone)]
pub struct RssFeedWithUnread {
    /// The feed.
    pub feed: RssFeed,
    /// Number of unread items for the current user.
    pub unread_count: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_rss_feed() {
        let feed = NewRssFeed::new("https://example.com/feed.xml", "Test Feed", 1);
        assert_eq!(feed.url, "https://example.com/feed.xml");
        assert_eq!(feed.title, "Test Feed");
        assert_eq!(feed.created_by, 1);
        assert!(feed.description.is_none());
    }

    #[test]
    fn test_new_rss_feed_with_description() {
        let feed = NewRssFeed::new("https://example.com/feed.xml", "Test Feed", 1)
            .with_description("A test feed")
            .with_site_url("https://example.com");
        assert_eq!(feed.description, Some("A test feed".to_string()));
        assert_eq!(feed.site_url, Some("https://example.com".to_string()));
    }

    #[test]
    fn test_rss_feed_is_due_for_fetch() {
        let feed = RssFeed {
            id: 1,
            url: "https://example.com/feed.xml".to_string(),
            title: "Test".to_string(),
            description: None,
            site_url: None,
            last_fetched_at: None,
            last_item_at: None,
            fetch_interval: 3600,
            is_active: true,
            error_count: 0,
            last_error: None,
            created_by: 1,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        // Never fetched, should be due
        assert!(feed.is_due_for_fetch());

        // Inactive feed should not be due
        let inactive = RssFeed {
            is_active: false,
            ..feed.clone()
        };
        assert!(!inactive.is_due_for_fetch());

        // Recently fetched should not be due
        let recent = RssFeed {
            last_fetched_at: Some(Utc::now()),
            ..feed.clone()
        };
        assert!(!recent.is_due_for_fetch());
    }

    #[test]
    fn test_rss_feed_error_threshold() {
        let feed = RssFeed {
            id: 1,
            url: "https://example.com/feed.xml".to_string(),
            title: "Test".to_string(),
            description: None,
            site_url: None,
            last_fetched_at: None,
            last_item_at: None,
            fetch_interval: 3600,
            is_active: true,
            error_count: 0,
            last_error: None,
            created_by: 1,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        assert!(!feed.has_exceeded_error_threshold());

        let errored = RssFeed {
            error_count: MAX_CONSECUTIVE_ERRORS,
            ..feed
        };
        assert!(errored.has_exceeded_error_threshold());
    }

    #[test]
    fn test_new_rss_item() {
        let item = NewRssItem::new(1, "guid-123", "Test Article");
        assert_eq!(item.feed_id, 1);
        assert_eq!(item.guid, "guid-123");
        assert_eq!(item.title, "Test Article");
    }

    #[test]
    fn test_new_rss_item_with_fields() {
        let now = Utc::now();
        let item = NewRssItem::new(1, "guid-123", "Test Article")
            .with_link("https://example.com/article")
            .with_description("Summary text")
            .with_author("Author Name")
            .with_published_at(now);
        assert_eq!(item.link, Some("https://example.com/article".to_string()));
        assert_eq!(item.description, Some("Summary text".to_string()));
        assert_eq!(item.author, Some("Author Name".to_string()));
        assert_eq!(item.published_at, Some(now));
    }

    #[test]
    fn test_new_rss_item_truncates_long_description() {
        let long_desc = "a".repeat(MAX_DESCRIPTION_LENGTH + 100);
        let item = NewRssItem::new(1, "guid-123", "Test").with_description(long_desc);
        assert_eq!(
            item.description.as_ref().unwrap().len(),
            MAX_DESCRIPTION_LENGTH
        );
    }

    #[test]
    fn test_rss_feed_update_empty() {
        let update = RssFeedUpdate::new();
        assert!(update.is_empty());
    }

    #[test]
    fn test_rss_feed_update_with_title() {
        let update = RssFeedUpdate::new().with_title("New Title");
        assert_eq!(update.title, Some("New Title".to_string()));
        assert!(!update.is_empty());
    }

    #[test]
    fn test_rss_feed_update_enable_disable() {
        let enable = RssFeedUpdate::new().enable();
        assert_eq!(enable.is_active, Some(true));

        let disable = RssFeedUpdate::new().disable();
        assert_eq!(disable.is_active, Some(false));
    }
}
