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

        let feed_repo = RssFeedRepository::new(self.db.pool());

        // Check if feed already exists
        if feed_repo.get_by_url(&request.url).await?.is_some() {
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

        let feed = feed_repo.create(&new_feed).await?;

        let item_repo = RssItemRepository::new(self.db.pool());

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
            item_repo.create_or_ignore(&new_item).await?;
        }

        // Update last_fetched_at
        feed_repo.clear_error(feed.id).await?;

        // Return updated feed
        feed_repo
            .get_by_id(feed.id)
            .await?
            .ok_or_else(|| HobbsError::NotFound("フィード".to_string()))
    }

    /// List all active feeds with unread counts.
    ///
    /// If user_id is provided, includes unread counts for that user.
    /// If user_id is None (guest), unread counts will be 0.
    pub async fn list_feeds(&self, user_id: Option<i64>) -> Result<Vec<RssFeedWithUnread>> {
        let feed_repo = RssFeedRepository::new(self.db.pool());
        let feeds = feed_repo.list_with_unread(user_id).await?;
        Ok(feeds)
    }

    /// Get a feed by ID.
    pub async fn get_feed(&self, feed_id: i64) -> Result<RssFeed> {
        let feed_repo = RssFeedRepository::new(self.db.pool());
        feed_repo
            .get_by_id(feed_id)
            .await?
            .ok_or_else(|| HobbsError::NotFound("フィード".to_string()))
    }

    /// Update a feed.
    ///
    /// Requires SubOp or higher permission.
    pub async fn update_feed(
        &self,
        feed_id: i64,
        update: &RssFeedUpdate,
        user: Option<&User>,
    ) -> Result<RssFeed> {
        // Check permission
        require_subop(user).map_err(|e| HobbsError::Permission(e.to_string()))?;

        // Check if feed exists
        let _ = self.get_feed(feed_id).await?;

        let feed_repo = RssFeedRepository::new(self.db.pool());

        // Update
        feed_repo.update(feed_id, update).await?;

        // Return updated feed
        self.get_feed(feed_id).await
    }

    /// Delete a feed.
    ///
    /// Requires SubOp or higher permission.
    /// This also deletes all items and read positions for the feed.
    pub async fn delete_feed(&self, feed_id: i64, user: Option<&User>) -> Result<()> {
        // Check permission
        require_subop(user).map_err(|e| HobbsError::Permission(e.to_string()))?;

        // Check if feed exists
        let _ = self.get_feed(feed_id).await?;

        let feed_repo = RssFeedRepository::new(self.db.pool());

        // Delete (cascades to items and read positions)
        feed_repo.delete(feed_id).await?;

        Ok(())
    }

    /// List items for a feed.
    ///
    /// Returns items sorted by published date (newest first).
    pub async fn list_items(
        &self,
        feed_id: i64,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<RssItem>> {
        // Check if feed exists
        let _ = self.get_feed(feed_id).await?;

        let item_repo = RssItemRepository::new(self.db.pool());
        let items = item_repo.list_by_feed(feed_id, limit, offset).await?;
        Ok(items)
    }

    /// Get an item by ID.
    ///
    /// If user_id is provided and logged in, updates the read position.
    pub async fn get_item(&self, item_id: i64, user_id: Option<i64>) -> Result<RssItem> {
        let item_repo = RssItemRepository::new(self.db.pool());
        let item = item_repo
            .get_by_id(item_id)
            .await?
            .ok_or_else(|| HobbsError::NotFound("記事".to_string()))?;

        // Update read position if user is logged in
        if let Some(uid) = user_id {
            let pos_repo = RssReadPositionRepository::new(self.db.pool());
            // Only update if this item is newer than current position
            let current = pos_repo.get(uid, item.feed_id).await?;
            let should_update = match current {
                None => true,
                Some(pos) => match pos.last_read_item_id {
                    None => true,
                    Some(last_id) => item.id > last_id,
                },
            };
            if should_update {
                pos_repo.upsert(uid, item.feed_id, item.id).await?;
            }
        }

        Ok(item)
    }

    /// Mark all items in a feed as read.
    ///
    /// Requires logged in user.
    pub async fn mark_all_as_read(&self, feed_id: i64, user_id: i64) -> Result<()> {
        // Check if feed exists
        let _ = self.get_feed(feed_id).await?;

        let pos_repo = RssReadPositionRepository::new(self.db.pool());
        pos_repo.mark_all_as_read(user_id, feed_id).await?;

        Ok(())
    }

    /// Count unread items for a user and feed.
    pub async fn count_unread(&self, feed_id: i64, user_id: i64) -> Result<i64> {
        let item_repo = RssItemRepository::new(self.db.pool());
        let count = item_repo.count_unread(feed_id, user_id).await?;
        Ok(count)
    }

    /// Count total unread items across all feeds for a user.
    pub async fn count_total_unread(&self, user_id: i64) -> Result<i64> {
        let feed_repo = RssFeedRepository::new(self.db.pool());
        let item_repo = RssItemRepository::new(self.db.pool());
        let feeds = feed_repo.list_active().await?;
        let mut total = 0i64;
        for feed in feeds {
            total += item_repo.count_unread(feed.id, user_id).await?;
        }
        Ok(total)
    }

    /// Refresh a feed (fetch new items).
    ///
    /// Requires SubOp or higher permission for manual refresh.
    pub async fn refresh_feed(&self, feed_id: i64, user: Option<&User>) -> Result<usize> {
        // Check permission
        require_subop(user).map_err(|e| HobbsError::Permission(e.to_string()))?;

        let feed = self.get_feed(feed_id).await?;

        let feed_repo = RssFeedRepository::new(self.db.pool());
        let item_repo = RssItemRepository::new(self.db.pool());

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

                    if item_repo.create_or_ignore(&new_item).await?.is_some() {
                        new_count += 1;
                    }
                }

                // Clear error and update last_fetched
                feed_repo.clear_error(feed_id).await?;

                // Prune old items
                item_repo.prune_old_items(feed_id).await?;

                Ok(new_count)
            }
            Err(e) => {
                // Increment error count
                feed_repo.increment_error(feed_id, &e.to_string()).await?;
                Err(e)
            }
        }
    }

    /// Check if a user has unread items in any feed.
    pub async fn has_unread(&self, user_id: i64) -> Result<bool> {
        let total = self.count_total_unread(user_id).await?;
        Ok(total > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{NewUser, Role, UserRepository};

    async fn setup_db() -> Database {
        Database::open_in_memory().await.unwrap()
    }

    async fn create_sysop(db: &Database) -> User {
        let repo = UserRepository::new(db.pool());
        let mut user = NewUser::new("sysop", "password123", "SysOp");
        user.role = Role::SysOp;
        repo.create(&user).await.unwrap()
    }

    async fn create_subop(db: &Database) -> User {
        let repo = UserRepository::new(db.pool());
        let mut user = NewUser::new("subop", "password123", "SubOp");
        user.role = Role::SubOp;
        repo.create(&user).await.unwrap()
    }

    async fn create_member(db: &Database) -> User {
        let repo = UserRepository::new(db.pool());
        let user = NewUser::new("member", "password123", "Member");
        repo.create(&user).await.unwrap()
    }

    async fn create_test_feed(db: &Database, user_id: i64) -> RssFeed {
        let feed_repo = RssFeedRepository::new(db.pool());
        let new_feed = NewRssFeed::new("https://example.com/feed.xml", "Test Feed", user_id);
        feed_repo.create(&new_feed).await.unwrap()
    }

    async fn create_test_items(db: &Database, feed_id: i64, count: usize) -> Vec<RssItem> {
        let item_repo = RssItemRepository::new(db.pool());
        let mut items = Vec::new();
        for i in 0..count {
            let item = NewRssItem::new(feed_id, format!("guid-{}", i), format!("Article {}", i));
            item_repo.create_or_ignore(&item).await.unwrap();
            let stored = item_repo
                .get_by_guid(feed_id, &format!("guid-{}", i))
                .await
                .unwrap()
                .unwrap();
            items.push(stored);
        }
        items
    }

    #[tokio::test]
    async fn test_list_feeds_empty() {
        let db = setup_db().await;
        let service = RssService::new(&db);

        let feeds = service.list_feeds(None).await.unwrap();
        assert!(feeds.is_empty());
    }

    #[tokio::test]
    async fn test_list_feeds_with_unread() {
        let db = setup_db().await;
        let member = create_member(&db).await;
        let service = RssService::new(&db);

        // Create feed and items for member (personal RSS model)
        let feed = create_test_feed(&db, member.id).await;
        create_test_items(&db, feed.id, 5).await;

        // List feeds for member (should show 5 unread)
        let feeds = service.list_feeds(Some(member.id)).await.unwrap();
        assert_eq!(feeds.len(), 1);
        assert_eq!(feeds[0].unread_count, 5);

        // Mark some as read
        let items = service.list_items(feed.id, 10, 0).await.unwrap();
        service.get_item(items[2].id, Some(member.id)).await.unwrap();

        // Now should have 2 unread (items 3 and 4, since we read up to 2)
        let feeds = service.list_feeds(Some(member.id)).await.unwrap();
        assert_eq!(feeds[0].unread_count, 2);
    }

    #[tokio::test]
    async fn test_list_feeds_guest() {
        let db = setup_db().await;
        let sysop = create_sysop(&db).await;
        let service = RssService::new(&db);

        let feed = create_test_feed(&db, sysop.id).await;
        create_test_items(&db, feed.id, 5).await;

        // Guest (None) cannot see any feeds in personal RSS model
        let feeds = service.list_feeds(None).await.unwrap();
        assert_eq!(feeds.len(), 0);
    }

    #[tokio::test]
    async fn test_get_feed() {
        let db = setup_db().await;
        let sysop = create_sysop(&db).await;
        let service = RssService::new(&db);

        let feed = create_test_feed(&db, sysop.id).await;
        let retrieved = service.get_feed(feed.id).await.unwrap();

        assert_eq!(retrieved.title, "Test Feed");
    }

    #[tokio::test]
    async fn test_get_feed_not_found() {
        let db = setup_db().await;
        let service = RssService::new(&db);

        let result = service.get_feed(999).await;
        assert!(matches!(result, Err(HobbsError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_update_feed_permission() {
        let db = setup_db().await;
        let sysop = create_sysop(&db).await;
        let member = create_member(&db).await;
        let service = RssService::new(&db);

        let feed = create_test_feed(&db, sysop.id).await;
        let update = RssFeedUpdate::new().with_title("Updated Title");

        // Member can't update
        let result = service.update_feed(feed.id, &update, Some(&member)).await;
        assert!(matches!(result, Err(HobbsError::Permission(_))));

        // SysOp can update
        let updated = service
            .update_feed(feed.id, &update, Some(&sysop))
            .await
            .unwrap();
        assert_eq!(updated.title, "Updated Title");
    }

    #[tokio::test]
    async fn test_update_feed_subop() {
        let db = setup_db().await;
        let sysop = create_sysop(&db).await;
        let subop = create_subop(&db).await;
        let service = RssService::new(&db);

        let feed = create_test_feed(&db, sysop.id).await;
        let update = RssFeedUpdate::new().with_title("SubOp Updated");

        // SubOp can update
        let updated = service
            .update_feed(feed.id, &update, Some(&subop))
            .await
            .unwrap();
        assert_eq!(updated.title, "SubOp Updated");
    }

    #[tokio::test]
    async fn test_delete_feed_permission() {
        let db = setup_db().await;
        let sysop = create_sysop(&db).await;
        let member = create_member(&db).await;
        let service = RssService::new(&db);

        let feed = create_test_feed(&db, sysop.id).await;

        // Member can't delete
        let result = service.delete_feed(feed.id, Some(&member)).await;
        assert!(matches!(result, Err(HobbsError::Permission(_))));

        // SysOp can delete
        service.delete_feed(feed.id, Some(&sysop)).await.unwrap();

        // Feed should be gone
        let result = service.get_feed(feed.id).await;
        assert!(matches!(result, Err(HobbsError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_list_items() {
        let db = setup_db().await;
        let sysop = create_sysop(&db).await;
        let service = RssService::new(&db);

        let feed = create_test_feed(&db, sysop.id).await;
        create_test_items(&db, feed.id, 10).await;

        let items = service.list_items(feed.id, 5, 0).await.unwrap();
        assert_eq!(items.len(), 5);

        let items2 = service.list_items(feed.id, 5, 5).await.unwrap();
        assert_eq!(items2.len(), 5);
    }

    #[tokio::test]
    async fn test_list_items_feed_not_found() {
        let db = setup_db().await;
        let service = RssService::new(&db);

        let result = service.list_items(999, 10, 0).await;
        assert!(matches!(result, Err(HobbsError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_get_item() {
        let db = setup_db().await;
        let sysop = create_sysop(&db).await;
        let service = RssService::new(&db);

        let feed = create_test_feed(&db, sysop.id).await;
        let items = create_test_items(&db, feed.id, 3).await;

        let item = service.get_item(items[1].id, None).await.unwrap();
        assert_eq!(item.title, "Article 1");
    }

    #[tokio::test]
    async fn test_get_item_updates_read_position() {
        let db = setup_db().await;
        let sysop = create_sysop(&db).await;
        let member = create_member(&db).await;
        let service = RssService::new(&db);

        let feed = create_test_feed(&db, sysop.id).await;
        let items = create_test_items(&db, feed.id, 5).await;

        // Initially 5 unread
        assert_eq!(service.count_unread(feed.id, member.id).await.unwrap(), 5);

        // Read item 2 (0-indexed)
        service.get_item(items[2].id, Some(member.id)).await.unwrap();

        // Now 2 unread (items 3 and 4)
        assert_eq!(service.count_unread(feed.id, member.id).await.unwrap(), 2);
    }

    #[tokio::test]
    async fn test_get_item_not_found() {
        let db = setup_db().await;
        let service = RssService::new(&db);

        let result = service.get_item(999, None).await;
        assert!(matches!(result, Err(HobbsError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_mark_all_as_read() {
        let db = setup_db().await;
        let sysop = create_sysop(&db).await;
        let member = create_member(&db).await;
        let service = RssService::new(&db);

        let feed = create_test_feed(&db, sysop.id).await;
        create_test_items(&db, feed.id, 5).await;

        assert_eq!(service.count_unread(feed.id, member.id).await.unwrap(), 5);

        service.mark_all_as_read(feed.id, member.id).await.unwrap();

        assert_eq!(service.count_unread(feed.id, member.id).await.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_count_total_unread() {
        let db = setup_db().await;
        let sysop = create_sysop(&db).await;
        let member = create_member(&db).await;
        let service = RssService::new(&db);

        // Create two feeds
        let feed1 = create_test_feed(&db, sysop.id).await;
        let feed_repo = RssFeedRepository::new(db.pool());
        let new_feed2 = NewRssFeed::new("https://example.com/feed2.xml", "Feed 2", sysop.id);
        let feed2 = feed_repo.create(&new_feed2).await.unwrap();

        create_test_items(&db, feed1.id, 3).await;
        create_test_items(&db, feed2.id, 2).await;

        assert_eq!(service.count_total_unread(member.id).await.unwrap(), 5);

        // Mark feed1 as read
        service.mark_all_as_read(feed1.id, member.id).await.unwrap();

        assert_eq!(service.count_total_unread(member.id).await.unwrap(), 2);
    }

    #[tokio::test]
    async fn test_has_unread() {
        let db = setup_db().await;
        let sysop = create_sysop(&db).await;
        let member = create_member(&db).await;
        let service = RssService::new(&db);

        assert!(!service.has_unread(member.id).await.unwrap());

        let feed = create_test_feed(&db, sysop.id).await;
        create_test_items(&db, feed.id, 3).await;

        assert!(service.has_unread(member.id).await.unwrap());

        service.mark_all_as_read(feed.id, member.id).await.unwrap();

        assert!(!service.has_unread(member.id).await.unwrap());
    }

    #[tokio::test]
    async fn test_delete_feed_cascades() {
        let db = setup_db().await;
        let sysop = create_sysop(&db).await;
        let member = create_member(&db).await;
        let service = RssService::new(&db);

        let feed = create_test_feed(&db, sysop.id).await;
        let items = create_test_items(&db, feed.id, 3).await;

        // Create read position
        service.get_item(items[1].id, Some(member.id)).await.unwrap();

        // Delete feed
        service.delete_feed(feed.id, Some(&sysop)).await.unwrap();

        // Items should be gone
        let item_repo = RssItemRepository::new(db.pool());
        let result = item_repo.get_by_id(items[0].id).await.unwrap();
        assert!(result.is_none());

        // Read position should be gone
        let pos_repo = RssReadPositionRepository::new(db.pool());
        let pos = pos_repo.get(member.id, feed.id).await.unwrap();
        assert!(pos.is_none());
    }
}
