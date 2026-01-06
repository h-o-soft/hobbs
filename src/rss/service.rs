//! RSS service for HOBBS.
//!
//! This module provides high-level RSS operations with business logic
//! including feed management, permission checks, and read position tracking.

use crate::auth::require_subop;
use crate::db::{Database, User};
use crate::rss::fetcher::fetch_feed;
use crate::rss::repository::{RssFeedRepository, RssItemRepository, RssReadPositionRepository};
use crate::rss::types::{
    NewRssFeed, NewRssItem, RssFeed, RssFeedUpdate, RssFeedWithUnread, RssItem, MAX_ITEMS_PER_FEED,
};
use crate::{HobbsError, Result};

/// Request to add a new RSS feed.
#[derive(Debug, Clone)]
pub struct AddFeedRequest {
    /// Feed URL.
    pub url: String,
    /// Custom title (optional, fetched from feed if not provided).
    pub title: Option<String>,
    /// User ID who is adding the feed.
    pub user_id: i64,
}

impl AddFeedRequest {
    /// Create a new add feed request.
    pub fn new(url: impl Into<String>, user_id: i64) -> Self {
        Self {
            url: url.into(),
            title: None,
            user_id,
        }
    }

    /// Set a custom title.
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }
}

/// Service for RSS operations.
pub struct RssService<'a> {
    db: &'a Database,
}

impl<'a> RssService<'a> {
    /// Create a new RssService with the given database reference.
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// Add a new RSS feed.
    ///
    /// Requires SubOp or higher permission.
    /// Fetches the feed to validate URL and get metadata.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - User doesn't have SubOp permission
    /// - URL is invalid or inaccessible
    /// - Feed already exists
    pub async fn add_feed(&self, request: &AddFeedRequest, user: Option<&User>) -> Result<RssFeed> {
        // Check permission
        require_subop(user).map_err(|e| HobbsError::Permission(e.to_string()))?;

        // Check if feed already exists
        if RssFeedRepository::get_by_url(self.db.conn(), &request.url)?.is_some() {
            return Err(HobbsError::Validation(
                "このURLのフィードは既に登録されています".to_string(),
            ));
        }

        // Fetch and parse the feed to validate
        let parsed = fetch_feed(&request.url).await?;

        // Use custom title or fetched title
        let title = request.title.clone().unwrap_or(parsed.title);

        // Create feed record
        let mut new_feed = NewRssFeed::new(&request.url, title, request.user_id);
        if let Some(desc) = parsed.description {
            new_feed = new_feed.with_description(desc);
        }
        if let Some(site_url) = parsed.site_url {
            new_feed = new_feed.with_site_url(site_url);
        }

        let feed = RssFeedRepository::create(self.db.conn(), &new_feed)?;

        // Store initial items
        for item in parsed.items.into_iter().take(MAX_ITEMS_PER_FEED) {
            let mut new_item = NewRssItem::new(feed.id, &item.guid, &item.title);
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
            RssItemRepository::create_or_ignore(self.db.conn(), &new_item)?;
        }

        // Update last_fetched_at
        RssFeedRepository::clear_error(self.db.conn(), feed.id)?;

        // Return updated feed
        RssFeedRepository::get_by_id(self.db.conn(), feed.id)?
            .ok_or_else(|| HobbsError::NotFound("フィード".to_string()))
    }

    /// List all active feeds with unread counts.
    ///
    /// If user_id is provided, includes unread counts for that user.
    /// If user_id is None (guest), unread counts will be 0.
    pub fn list_feeds(&self, user_id: Option<i64>) -> Result<Vec<RssFeedWithUnread>> {
        let feeds = RssFeedRepository::list_with_unread(self.db.conn(), user_id)?;
        Ok(feeds)
    }

    /// Get a feed by ID.
    pub fn get_feed(&self, feed_id: i64) -> Result<RssFeed> {
        RssFeedRepository::get_by_id(self.db.conn(), feed_id)?
            .ok_or_else(|| HobbsError::NotFound("フィード".to_string()))
    }

    /// Update a feed.
    ///
    /// Requires SubOp or higher permission.
    pub fn update_feed(
        &self,
        feed_id: i64,
        update: &RssFeedUpdate,
        user: Option<&User>,
    ) -> Result<RssFeed> {
        // Check permission
        require_subop(user).map_err(|e| HobbsError::Permission(e.to_string()))?;

        // Check if feed exists
        let _ = self.get_feed(feed_id)?;

        // Update
        RssFeedRepository::update(self.db.conn(), feed_id, update)?;

        // Return updated feed
        self.get_feed(feed_id)
    }

    /// Delete a feed.
    ///
    /// Requires SubOp or higher permission.
    /// This also deletes all items and read positions for the feed.
    pub fn delete_feed(&self, feed_id: i64, user: Option<&User>) -> Result<()> {
        // Check permission
        require_subop(user).map_err(|e| HobbsError::Permission(e.to_string()))?;

        // Check if feed exists
        let _ = self.get_feed(feed_id)?;

        // Delete (cascades to items and read positions)
        RssFeedRepository::delete(self.db.conn(), feed_id)?;

        Ok(())
    }

    /// List items for a feed.
    ///
    /// Returns items sorted by published date (newest first).
    pub fn list_items(&self, feed_id: i64, limit: usize, offset: usize) -> Result<Vec<RssItem>> {
        // Check if feed exists
        let _ = self.get_feed(feed_id)?;

        let items = RssItemRepository::list_by_feed(self.db.conn(), feed_id, limit, offset)?;
        Ok(items)
    }

    /// Get an item by ID.
    ///
    /// If user_id is provided and logged in, updates the read position.
    pub fn get_item(&self, item_id: i64, user_id: Option<i64>) -> Result<RssItem> {
        let item = RssItemRepository::get_by_id(self.db.conn(), item_id)?
            .ok_or_else(|| HobbsError::NotFound("記事".to_string()))?;

        // Update read position if user is logged in
        if let Some(uid) = user_id {
            // Only update if this item is newer than current position
            let current = RssReadPositionRepository::get(self.db.conn(), uid, item.feed_id)?;
            let should_update = match current {
                None => true,
                Some(pos) => match pos.last_read_item_id {
                    None => true,
                    Some(last_id) => item.id > last_id,
                },
            };
            if should_update {
                RssReadPositionRepository::upsert(self.db.conn(), uid, item.feed_id, item.id)?;
            }
        }

        Ok(item)
    }

    /// Mark all items in a feed as read.
    ///
    /// Requires logged in user.
    pub fn mark_all_as_read(&self, feed_id: i64, user_id: i64) -> Result<()> {
        // Check if feed exists
        let _ = self.get_feed(feed_id)?;

        RssReadPositionRepository::mark_all_as_read(self.db.conn(), user_id, feed_id)?;

        Ok(())
    }

    /// Count unread items for a user and feed.
    pub fn count_unread(&self, feed_id: i64, user_id: i64) -> Result<i64> {
        let count = RssItemRepository::count_unread(self.db.conn(), feed_id, user_id)?;
        Ok(count)
    }

    /// Count total unread items across all feeds for a user.
    pub fn count_total_unread(&self, user_id: i64) -> Result<i64> {
        let feeds = RssFeedRepository::list_active(self.db.conn())?;
        let mut total = 0i64;
        for feed in feeds {
            total += RssItemRepository::count_unread(self.db.conn(), feed.id, user_id)?;
        }
        Ok(total)
    }

    /// Refresh a feed (fetch new items).
    ///
    /// Requires SubOp or higher permission for manual refresh.
    pub async fn refresh_feed(&self, feed_id: i64, user: Option<&User>) -> Result<usize> {
        // Check permission
        require_subop(user).map_err(|e| HobbsError::Permission(e.to_string()))?;

        let feed = self.get_feed(feed_id)?;

        // Fetch and parse the feed
        match fetch_feed(&feed.url).await {
            Ok(parsed) => {
                let mut new_count = 0;

                // Store items
                for item in parsed.items.into_iter().take(MAX_ITEMS_PER_FEED) {
                    let mut new_item = NewRssItem::new(feed.id, &item.guid, &item.title);
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

                    if RssItemRepository::create_or_ignore(self.db.conn(), &new_item)?.is_some() {
                        new_count += 1;
                    }
                }

                // Clear error and update last_fetched
                RssFeedRepository::clear_error(self.db.conn(), feed_id)?;

                // Prune old items
                RssItemRepository::prune_old_items(self.db.conn(), feed_id)?;

                Ok(new_count)
            }
            Err(e) => {
                // Increment error count
                RssFeedRepository::increment_error(self.db.conn(), feed_id, &e.to_string())?;
                Err(e)
            }
        }
    }

    /// Check if a user has unread items in any feed.
    pub fn has_unread(&self, user_id: i64) -> Result<bool> {
        let total = self.count_total_unread(user_id)?;
        Ok(total > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{NewUser, Role, UserRepository};

    fn setup_db() -> Database {
        Database::open_in_memory().unwrap()
    }

    fn create_sysop(db: &Database) -> User {
        let repo = UserRepository::new(db);
        let mut user = NewUser::new("sysop", "password123", "SysOp");
        user.role = Role::SysOp;
        repo.create(&user).unwrap()
    }

    fn create_subop(db: &Database) -> User {
        let repo = UserRepository::new(db);
        let mut user = NewUser::new("subop", "password123", "SubOp");
        user.role = Role::SubOp;
        repo.create(&user).unwrap()
    }

    fn create_member(db: &Database) -> User {
        let repo = UserRepository::new(db);
        let user = NewUser::new("member", "password123", "Member");
        repo.create(&user).unwrap()
    }

    fn create_test_feed(db: &Database, user_id: i64) -> RssFeed {
        let new_feed = NewRssFeed::new("https://example.com/feed.xml", "Test Feed", user_id);
        RssFeedRepository::create(db.conn(), &new_feed).unwrap()
    }

    fn create_test_items(db: &Database, feed_id: i64, count: usize) -> Vec<RssItem> {
        let mut items = Vec::new();
        for i in 0..count {
            let item = NewRssItem::new(feed_id, format!("guid-{}", i), format!("Article {}", i));
            RssItemRepository::create_or_ignore(db.conn(), &item).unwrap();
            let stored = RssItemRepository::get_by_guid(db.conn(), feed_id, &format!("guid-{}", i))
                .unwrap()
                .unwrap();
            items.push(stored);
        }
        items
    }

    #[test]
    fn test_list_feeds_empty() {
        let db = setup_db();
        let service = RssService::new(&db);

        let feeds = service.list_feeds(None).unwrap();
        assert!(feeds.is_empty());
    }

    #[test]
    fn test_list_feeds_with_unread() {
        let db = setup_db();
        let member = create_member(&db);
        let service = RssService::new(&db);

        // Create feed and items for member (personal RSS model)
        let feed = create_test_feed(&db, member.id);
        create_test_items(&db, feed.id, 5);

        // List feeds for member (should show 5 unread)
        let feeds = service.list_feeds(Some(member.id)).unwrap();
        assert_eq!(feeds.len(), 1);
        assert_eq!(feeds[0].unread_count, 5);

        // Mark some as read
        let items = service.list_items(feed.id, 10, 0).unwrap();
        service.get_item(items[2].id, Some(member.id)).unwrap();

        // Now should have 2 unread (items 3 and 4, since we read up to 2)
        let feeds = service.list_feeds(Some(member.id)).unwrap();
        assert_eq!(feeds[0].unread_count, 2);
    }

    #[test]
    fn test_list_feeds_guest() {
        let db = setup_db();
        let sysop = create_sysop(&db);
        let service = RssService::new(&db);

        let feed = create_test_feed(&db, sysop.id);
        create_test_items(&db, feed.id, 5);

        // Guest (None) cannot see any feeds in personal RSS model
        let feeds = service.list_feeds(None).unwrap();
        assert_eq!(feeds.len(), 0);
    }

    #[test]
    fn test_get_feed() {
        let db = setup_db();
        let sysop = create_sysop(&db);
        let service = RssService::new(&db);

        let feed = create_test_feed(&db, sysop.id);
        let retrieved = service.get_feed(feed.id).unwrap();

        assert_eq!(retrieved.title, "Test Feed");
    }

    #[test]
    fn test_get_feed_not_found() {
        let db = setup_db();
        let service = RssService::new(&db);

        let result = service.get_feed(999);
        assert!(matches!(result, Err(HobbsError::NotFound(_))));
    }

    #[test]
    fn test_update_feed_permission() {
        let db = setup_db();
        let sysop = create_sysop(&db);
        let member = create_member(&db);
        let service = RssService::new(&db);

        let feed = create_test_feed(&db, sysop.id);
        let update = RssFeedUpdate::new().with_title("Updated Title");

        // Member can't update
        let result = service.update_feed(feed.id, &update, Some(&member));
        assert!(matches!(result, Err(HobbsError::Permission(_))));

        // SysOp can update
        let updated = service.update_feed(feed.id, &update, Some(&sysop)).unwrap();
        assert_eq!(updated.title, "Updated Title");
    }

    #[test]
    fn test_update_feed_subop() {
        let db = setup_db();
        let sysop = create_sysop(&db);
        let subop = create_subop(&db);
        let service = RssService::new(&db);

        let feed = create_test_feed(&db, sysop.id);
        let update = RssFeedUpdate::new().with_title("SubOp Updated");

        // SubOp can update
        let updated = service.update_feed(feed.id, &update, Some(&subop)).unwrap();
        assert_eq!(updated.title, "SubOp Updated");
    }

    #[test]
    fn test_delete_feed_permission() {
        let db = setup_db();
        let sysop = create_sysop(&db);
        let member = create_member(&db);
        let service = RssService::new(&db);

        let feed = create_test_feed(&db, sysop.id);

        // Member can't delete
        let result = service.delete_feed(feed.id, Some(&member));
        assert!(matches!(result, Err(HobbsError::Permission(_))));

        // SysOp can delete
        service.delete_feed(feed.id, Some(&sysop)).unwrap();

        // Feed should be gone
        let result = service.get_feed(feed.id);
        assert!(matches!(result, Err(HobbsError::NotFound(_))));
    }

    #[test]
    fn test_list_items() {
        let db = setup_db();
        let sysop = create_sysop(&db);
        let service = RssService::new(&db);

        let feed = create_test_feed(&db, sysop.id);
        create_test_items(&db, feed.id, 10);

        let items = service.list_items(feed.id, 5, 0).unwrap();
        assert_eq!(items.len(), 5);

        let items2 = service.list_items(feed.id, 5, 5).unwrap();
        assert_eq!(items2.len(), 5);
    }

    #[test]
    fn test_list_items_feed_not_found() {
        let db = setup_db();
        let service = RssService::new(&db);

        let result = service.list_items(999, 10, 0);
        assert!(matches!(result, Err(HobbsError::NotFound(_))));
    }

    #[test]
    fn test_get_item() {
        let db = setup_db();
        let sysop = create_sysop(&db);
        let service = RssService::new(&db);

        let feed = create_test_feed(&db, sysop.id);
        let items = create_test_items(&db, feed.id, 3);

        let item = service.get_item(items[1].id, None).unwrap();
        assert_eq!(item.title, "Article 1");
    }

    #[test]
    fn test_get_item_updates_read_position() {
        let db = setup_db();
        let sysop = create_sysop(&db);
        let member = create_member(&db);
        let service = RssService::new(&db);

        let feed = create_test_feed(&db, sysop.id);
        let items = create_test_items(&db, feed.id, 5);

        // Initially 5 unread
        assert_eq!(service.count_unread(feed.id, member.id).unwrap(), 5);

        // Read item 2 (0-indexed)
        service.get_item(items[2].id, Some(member.id)).unwrap();

        // Now 2 unread (items 3 and 4)
        assert_eq!(service.count_unread(feed.id, member.id).unwrap(), 2);
    }

    #[test]
    fn test_get_item_not_found() {
        let db = setup_db();
        let service = RssService::new(&db);

        let result = service.get_item(999, None);
        assert!(matches!(result, Err(HobbsError::NotFound(_))));
    }

    #[test]
    fn test_mark_all_as_read() {
        let db = setup_db();
        let sysop = create_sysop(&db);
        let member = create_member(&db);
        let service = RssService::new(&db);

        let feed = create_test_feed(&db, sysop.id);
        create_test_items(&db, feed.id, 5);

        assert_eq!(service.count_unread(feed.id, member.id).unwrap(), 5);

        service.mark_all_as_read(feed.id, member.id).unwrap();

        assert_eq!(service.count_unread(feed.id, member.id).unwrap(), 0);
    }

    #[test]
    fn test_count_total_unread() {
        let db = setup_db();
        let sysop = create_sysop(&db);
        let member = create_member(&db);
        let service = RssService::new(&db);

        // Create two feeds
        let feed1 = create_test_feed(&db, sysop.id);
        let new_feed2 = NewRssFeed::new("https://example.com/feed2.xml", "Feed 2", sysop.id);
        let feed2 = RssFeedRepository::create(db.conn(), &new_feed2).unwrap();

        create_test_items(&db, feed1.id, 3);
        create_test_items(&db, feed2.id, 2);

        assert_eq!(service.count_total_unread(member.id).unwrap(), 5);

        // Mark feed1 as read
        service.mark_all_as_read(feed1.id, member.id).unwrap();

        assert_eq!(service.count_total_unread(member.id).unwrap(), 2);
    }

    #[test]
    fn test_has_unread() {
        let db = setup_db();
        let sysop = create_sysop(&db);
        let member = create_member(&db);
        let service = RssService::new(&db);

        assert!(!service.has_unread(member.id).unwrap());

        let feed = create_test_feed(&db, sysop.id);
        create_test_items(&db, feed.id, 3);

        assert!(service.has_unread(member.id).unwrap());

        service.mark_all_as_read(feed.id, member.id).unwrap();

        assert!(!service.has_unread(member.id).unwrap());
    }

    #[test]
    fn test_delete_feed_cascades() {
        let db = setup_db();
        let sysop = create_sysop(&db);
        let member = create_member(&db);
        let service = RssService::new(&db);

        let feed = create_test_feed(&db, sysop.id);
        let items = create_test_items(&db, feed.id, 3);

        // Create read position
        service.get_item(items[1].id, Some(member.id)).unwrap();

        // Delete feed
        service.delete_feed(feed.id, Some(&sysop)).unwrap();

        // Items should be gone
        let result = RssItemRepository::get_by_id(db.conn(), items[0].id).unwrap();
        assert!(result.is_none());

        // Read position should be gone
        let pos = RssReadPositionRepository::get(db.conn(), member.id, feed.id).unwrap();
        assert!(pos.is_none());
    }
}
