//! RSS repositories for HOBBS.

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};

use super::types::{
    NewRssFeed, NewRssItem, RssFeed, RssFeedUpdate, RssFeedWithUnread, RssItem, RssReadPosition,
    MAX_ITEMS_PER_FEED,
};

/// Repository for RSS feed operations.
pub struct RssFeedRepository;

impl RssFeedRepository {
    /// Create a new feed.
    pub fn create(conn: &Connection, feed: &NewRssFeed) -> rusqlite::Result<RssFeed> {
        conn.execute(
            r#"
            INSERT INTO rss_feeds (url, title, description, site_url, created_by)
            VALUES (?1, ?2, ?3, ?4, ?5)
            "#,
            params![
                feed.url,
                feed.title,
                feed.description,
                feed.site_url,
                feed.created_by
            ],
        )?;

        let id = conn.last_insert_rowid();
        Self::get_by_id(conn, id)?.ok_or_else(|| rusqlite::Error::QueryReturnedNoRows)
    }

    /// Get a feed by ID.
    pub fn get_by_id(conn: &Connection, id: i64) -> rusqlite::Result<Option<RssFeed>> {
        conn.query_row(
            r#"
            SELECT id, url, title, description, site_url, last_fetched_at, last_item_at,
                   fetch_interval, is_active, error_count, last_error, created_by,
                   created_at, updated_at
            FROM rss_feeds
            WHERE id = ?1
            "#,
            [id],
            Self::map_row,
        )
        .optional()
    }

    /// Get a feed by URL.
    pub fn get_by_url(conn: &Connection, url: &str) -> rusqlite::Result<Option<RssFeed>> {
        conn.query_row(
            r#"
            SELECT id, url, title, description, site_url, last_fetched_at, last_item_at,
                   fetch_interval, is_active, error_count, last_error, created_by,
                   created_at, updated_at
            FROM rss_feeds
            WHERE url = ?1
            "#,
            [url],
            Self::map_row,
        )
        .optional()
    }

    /// List all active feeds.
    pub fn list_active(conn: &Connection) -> rusqlite::Result<Vec<RssFeed>> {
        let mut stmt = conn.prepare(
            r#"
            SELECT id, url, title, description, site_url, last_fetched_at, last_item_at,
                   fetch_interval, is_active, error_count, last_error, created_by,
                   created_at, updated_at
            FROM rss_feeds
            WHERE is_active = 1
            ORDER BY title ASC
            "#,
        )?;

        let feeds = stmt
            .query_map([], Self::map_row)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(feeds)
    }

    /// List all feeds (including inactive).
    pub fn list_all(conn: &Connection) -> rusqlite::Result<Vec<RssFeed>> {
        let mut stmt = conn.prepare(
            r#"
            SELECT id, url, title, description, site_url, last_fetched_at, last_item_at,
                   fetch_interval, is_active, error_count, last_error, created_by,
                   created_at, updated_at
            FROM rss_feeds
            ORDER BY title ASC
            "#,
        )?;

        let feeds = stmt
            .query_map([], Self::map_row)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(feeds)
    }

    /// List feeds that are due for fetching.
    pub fn list_due_for_fetch(conn: &Connection) -> rusqlite::Result<Vec<RssFeed>> {
        let mut stmt = conn.prepare(
            r#"
            SELECT id, url, title, description, site_url, last_fetched_at, last_item_at,
                   fetch_interval, is_active, error_count, last_error, created_by,
                   created_at, updated_at
            FROM rss_feeds
            WHERE is_active = 1
              AND (last_fetched_at IS NULL
                   OR datetime(last_fetched_at, '+' || fetch_interval || ' seconds') <= datetime('now'))
            ORDER BY last_fetched_at ASC NULLS FIRST
            "#,
        )?;

        let feeds = stmt
            .query_map([], Self::map_row)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(feeds)
    }

    /// List active feeds with unread counts for a user.
    pub fn list_with_unread(
        conn: &Connection,
        user_id: Option<i64>,
    ) -> rusqlite::Result<Vec<RssFeedWithUnread>> {
        let mut stmt = conn.prepare(
            r#"
            SELECT f.id, f.url, f.title, f.description, f.site_url, f.last_fetched_at, f.last_item_at,
                   f.fetch_interval, f.is_active, f.error_count, f.last_error, f.created_by,
                   f.created_at, f.updated_at,
                   CASE WHEN ?1 IS NULL THEN 0
                        ELSE (SELECT COUNT(*) FROM rss_items i
                              WHERE i.feed_id = f.id
                              AND i.id > COALESCE(
                                  (SELECT last_read_item_id FROM rss_read_positions
                                   WHERE user_id = ?1 AND feed_id = f.id),
                                  0))
                   END as unread_count
            FROM rss_feeds f
            WHERE f.is_active = 1
            ORDER BY f.title ASC
            "#,
        )?;

        let feeds = stmt
            .query_map([user_id], |row| {
                let feed = Self::map_row(row)?;
                let unread_count: i64 = row.get(14)?;
                Ok(RssFeedWithUnread { feed, unread_count })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(feeds)
    }

    /// Update a feed.
    pub fn update(conn: &Connection, id: i64, update: &RssFeedUpdate) -> rusqlite::Result<bool> {
        if update.is_empty() {
            return Ok(false);
        }

        let mut sets = Vec::new();
        let mut values: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(ref title) = update.title {
            sets.push("title = ?");
            values.push(Box::new(title.clone()));
        }

        if let Some(ref description) = update.description {
            sets.push("description = ?");
            values.push(Box::new(description.clone()));
        }

        if let Some(interval) = update.fetch_interval {
            sets.push("fetch_interval = ?");
            values.push(Box::new(interval));
        }

        if let Some(is_active) = update.is_active {
            sets.push("is_active = ?");
            values.push(Box::new(is_active as i32));
        }

        sets.push("updated_at = datetime('now')");
        values.push(Box::new(id));

        let sql = format!("UPDATE rss_feeds SET {} WHERE id = ?", sets.join(", "));

        let params: Vec<&dyn rusqlite::ToSql> = values.iter().map(|v| v.as_ref()).collect();
        let rows = conn.execute(&sql, params.as_slice())?;
        Ok(rows > 0)
    }

    /// Update last fetched timestamp.
    pub fn update_last_fetched(conn: &Connection, id: i64) -> rusqlite::Result<bool> {
        let rows = conn.execute(
            "UPDATE rss_feeds SET last_fetched_at = datetime('now'), updated_at = datetime('now') WHERE id = ?1",
            [id],
        )?;
        Ok(rows > 0)
    }

    /// Update last item timestamp.
    pub fn update_last_item_at(
        conn: &Connection,
        id: i64,
        last_item_at: DateTime<Utc>,
    ) -> rusqlite::Result<bool> {
        let rows = conn.execute(
            "UPDATE rss_feeds SET last_item_at = ?2, updated_at = datetime('now') WHERE id = ?1",
            params![id, last_item_at.to_rfc3339()],
        )?;
        Ok(rows > 0)
    }

    /// Increment error count and set error message.
    pub fn increment_error(conn: &Connection, id: i64, error: &str) -> rusqlite::Result<bool> {
        let rows = conn.execute(
            r#"
            UPDATE rss_feeds
            SET error_count = error_count + 1,
                last_error = ?2,
                updated_at = datetime('now')
            WHERE id = ?1
            "#,
            params![id, error],
        )?;
        Ok(rows > 0)
    }

    /// Clear error count.
    pub fn clear_error(conn: &Connection, id: i64) -> rusqlite::Result<bool> {
        let rows = conn.execute(
            r#"
            UPDATE rss_feeds
            SET error_count = 0,
                last_error = NULL,
                updated_at = datetime('now')
            WHERE id = ?1
            "#,
            [id],
        )?;
        Ok(rows > 0)
    }

    /// Disable feeds that have exceeded the error threshold.
    pub fn disable_failed_feeds(conn: &Connection, max_errors: i32) -> rusqlite::Result<usize> {
        conn.execute(
            r#"
            UPDATE rss_feeds
            SET is_active = 0, updated_at = datetime('now')
            WHERE error_count >= ?1 AND is_active = 1
            "#,
            [max_errors],
        )
    }

    /// Delete a feed.
    pub fn delete(conn: &Connection, id: i64) -> rusqlite::Result<bool> {
        let rows = conn.execute("DELETE FROM rss_feeds WHERE id = ?1", [id])?;
        Ok(rows > 0)
    }

    /// Count all feeds.
    pub fn count(conn: &Connection) -> rusqlite::Result<i64> {
        conn.query_row("SELECT COUNT(*) FROM rss_feeds", [], |row| row.get(0))
    }

    /// Map a database row to an RssFeed.
    fn map_row(row: &rusqlite::Row) -> rusqlite::Result<RssFeed> {
        let last_fetched_at: Option<String> = row.get(5)?;
        let last_item_at: Option<String> = row.get(6)?;
        let created_at_str: String = row.get(12)?;
        let updated_at_str: String = row.get(13)?;

        Ok(RssFeed {
            id: row.get(0)?,
            url: row.get(1)?,
            title: row.get(2)?,
            description: row.get(3)?,
            site_url: row.get(4)?,
            last_fetched_at: last_fetched_at.and_then(|s| parse_datetime(&s)),
            last_item_at: last_item_at.and_then(|s| parse_datetime(&s)),
            fetch_interval: row.get(7)?,
            is_active: row.get::<_, i32>(8)? != 0,
            error_count: row.get(9)?,
            last_error: row.get(10)?,
            created_by: row.get(11)?,
            created_at: parse_datetime(&created_at_str).unwrap_or_else(Utc::now),
            updated_at: parse_datetime(&updated_at_str).unwrap_or_else(Utc::now),
        })
    }
}

/// Repository for RSS item operations.
pub struct RssItemRepository;

impl RssItemRepository {
    /// Create a new item, ignoring if duplicate (same feed_id + guid).
    pub fn create_or_ignore(conn: &Connection, item: &NewRssItem) -> rusqlite::Result<Option<i64>> {
        let published_at = item.published_at.map(|dt| dt.to_rfc3339());

        let rows = conn.execute(
            r#"
            INSERT OR IGNORE INTO rss_items (feed_id, guid, title, link, description, author, published_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            "#,
            params![
                item.feed_id,
                item.guid,
                item.title,
                item.link,
                item.description,
                item.author,
                published_at
            ],
        )?;

        if rows > 0 {
            Ok(Some(conn.last_insert_rowid()))
        } else {
            Ok(None) // Already existed
        }
    }

    /// Get an item by ID.
    pub fn get_by_id(conn: &Connection, id: i64) -> rusqlite::Result<Option<RssItem>> {
        conn.query_row(
            r#"
            SELECT id, feed_id, guid, title, link, description, author, published_at, fetched_at
            FROM rss_items
            WHERE id = ?1
            "#,
            [id],
            Self::map_row,
        )
        .optional()
    }

    /// Get an item by feed ID and guid.
    pub fn get_by_guid(
        conn: &Connection,
        feed_id: i64,
        guid: &str,
    ) -> rusqlite::Result<Option<RssItem>> {
        conn.query_row(
            r#"
            SELECT id, feed_id, guid, title, link, description, author, published_at, fetched_at
            FROM rss_items
            WHERE feed_id = ?1 AND guid = ?2
            "#,
            params![feed_id, guid],
            Self::map_row,
        )
        .optional()
    }

    /// List items for a feed (newest first).
    pub fn list_by_feed(
        conn: &Connection,
        feed_id: i64,
        limit: usize,
        offset: usize,
    ) -> rusqlite::Result<Vec<RssItem>> {
        let mut stmt = conn.prepare(
            r#"
            SELECT id, feed_id, guid, title, link, description, author, published_at, fetched_at
            FROM rss_items
            WHERE feed_id = ?1
            ORDER BY COALESCE(published_at, fetched_at) DESC, id DESC
            LIMIT ?2 OFFSET ?3
            "#,
        )?;

        let items = stmt
            .query_map(params![feed_id, limit as i64, offset as i64], Self::map_row)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(items)
    }

    /// Count items for a feed.
    pub fn count_by_feed(conn: &Connection, feed_id: i64) -> rusqlite::Result<i64> {
        conn.query_row(
            "SELECT COUNT(*) FROM rss_items WHERE feed_id = ?1",
            [feed_id],
            |row| row.get(0),
        )
    }

    /// Count unread items for a user and feed.
    pub fn count_unread(conn: &Connection, feed_id: i64, user_id: i64) -> rusqlite::Result<i64> {
        conn.query_row(
            r#"
            SELECT COUNT(*) FROM rss_items
            WHERE feed_id = ?1
            AND id > COALESCE(
                (SELECT last_read_item_id FROM rss_read_positions
                 WHERE user_id = ?2 AND feed_id = ?1),
                0)
            "#,
            params![feed_id, user_id],
            |row| row.get(0),
        )
    }

    /// Get the newest item ID for a feed.
    pub fn get_newest_item_id(conn: &Connection, feed_id: i64) -> rusqlite::Result<Option<i64>> {
        conn.query_row(
            r#"
            SELECT id FROM rss_items
            WHERE feed_id = ?1
            ORDER BY COALESCE(published_at, fetched_at) DESC, id DESC
            LIMIT 1
            "#,
            [feed_id],
            |row| row.get(0),
        )
        .optional()
    }

    /// Delete old items for a feed, keeping only the most recent.
    pub fn prune_old_items(conn: &Connection, feed_id: i64) -> rusqlite::Result<usize> {
        conn.execute(
            r#"
            DELETE FROM rss_items
            WHERE feed_id = ?1
            AND id NOT IN (
                SELECT id FROM rss_items
                WHERE feed_id = ?1
                ORDER BY COALESCE(published_at, fetched_at) DESC, id DESC
                LIMIT ?2
            )
            "#,
            params![feed_id, MAX_ITEMS_PER_FEED as i64],
        )
    }

    /// Delete all items for a feed.
    pub fn delete_by_feed(conn: &Connection, feed_id: i64) -> rusqlite::Result<usize> {
        conn.execute("DELETE FROM rss_items WHERE feed_id = ?1", [feed_id])
    }

    /// Map a database row to an RssItem.
    fn map_row(row: &rusqlite::Row) -> rusqlite::Result<RssItem> {
        let published_at: Option<String> = row.get(7)?;
        let fetched_at_str: String = row.get(8)?;

        Ok(RssItem {
            id: row.get(0)?,
            feed_id: row.get(1)?,
            guid: row.get(2)?,
            title: row.get(3)?,
            link: row.get(4)?,
            description: row.get(5)?,
            author: row.get(6)?,
            published_at: published_at.and_then(|s| parse_datetime(&s)),
            fetched_at: parse_datetime(&fetched_at_str).unwrap_or_else(Utc::now),
        })
    }
}

/// Repository for RSS read position operations.
pub struct RssReadPositionRepository;

impl RssReadPositionRepository {
    /// Get read position for a user and feed.
    pub fn get(
        conn: &Connection,
        user_id: i64,
        feed_id: i64,
    ) -> rusqlite::Result<Option<RssReadPosition>> {
        conn.query_row(
            r#"
            SELECT id, user_id, feed_id, last_read_item_id, last_read_at
            FROM rss_read_positions
            WHERE user_id = ?1 AND feed_id = ?2
            "#,
            params![user_id, feed_id],
            Self::map_row,
        )
        .optional()
    }

    /// Update or insert read position.
    pub fn upsert(
        conn: &Connection,
        user_id: i64,
        feed_id: i64,
        last_read_item_id: i64,
    ) -> rusqlite::Result<()> {
        conn.execute(
            r#"
            INSERT INTO rss_read_positions (user_id, feed_id, last_read_item_id, last_read_at)
            VALUES (?1, ?2, ?3, datetime('now'))
            ON CONFLICT(user_id, feed_id) DO UPDATE SET
                last_read_item_id = ?3,
                last_read_at = datetime('now')
            "#,
            params![user_id, feed_id, last_read_item_id],
        )?;
        Ok(())
    }

    /// Mark all items as read (set to newest item ID).
    pub fn mark_all_as_read(
        conn: &Connection,
        user_id: i64,
        feed_id: i64,
    ) -> rusqlite::Result<bool> {
        let newest_id = RssItemRepository::get_newest_item_id(conn, feed_id)?;
        match newest_id {
            Some(id) => {
                Self::upsert(conn, user_id, feed_id, id)?;
                Ok(true)
            }
            None => Ok(false), // No items to mark as read
        }
    }

    /// Delete read position for a user and feed.
    pub fn delete(conn: &Connection, user_id: i64, feed_id: i64) -> rusqlite::Result<bool> {
        let rows = conn.execute(
            "DELETE FROM rss_read_positions WHERE user_id = ?1 AND feed_id = ?2",
            params![user_id, feed_id],
        )?;
        Ok(rows > 0)
    }

    /// Delete all read positions for a user.
    pub fn delete_by_user(conn: &Connection, user_id: i64) -> rusqlite::Result<usize> {
        conn.execute(
            "DELETE FROM rss_read_positions WHERE user_id = ?1",
            [user_id],
        )
    }

    /// Map a database row to an RssReadPosition.
    fn map_row(row: &rusqlite::Row) -> rusqlite::Result<RssReadPosition> {
        let last_read_at_str: String = row.get(4)?;

        Ok(RssReadPosition {
            id: row.get(0)?,
            user_id: row.get(1)?,
            feed_id: row.get(2)?,
            last_read_item_id: row.get(3)?,
            last_read_at: parse_datetime(&last_read_at_str).unwrap_or_else(Utc::now),
        })
    }
}

/// Parse a datetime string to DateTime<Utc>.
fn parse_datetime(s: &str) -> Option<DateTime<Utc>> {
    // Try RFC3339 first
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Some(dt.with_timezone(&Utc));
    }
    // Try SQLite datetime format
    if let Ok(naive) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S") {
        return Some(DateTime::from_naive_utc_and_offset(naive, Utc));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{Database, NewUser, UserRepository};

    fn setup_db() -> Database {
        Database::open_in_memory().unwrap()
    }

    fn create_test_user(db: &Database) -> i64 {
        let repo = UserRepository::new(db);
        let user = NewUser::new("testuser", "password123", "Test User");
        repo.create(&user).unwrap().id
    }

    #[test]
    fn test_create_feed() {
        let db = setup_db();
        let user_id = create_test_user(&db);

        let new_feed = NewRssFeed::new("https://example.com/feed.xml", "Test Feed", user_id);
        let feed = RssFeedRepository::create(db.conn(), &new_feed).unwrap();

        assert!(feed.id > 0);
        assert_eq!(feed.url, "https://example.com/feed.xml");
        assert_eq!(feed.title, "Test Feed");
        assert_eq!(feed.created_by, user_id);
        assert!(feed.is_active);
        assert_eq!(feed.error_count, 0);
    }

    #[test]
    fn test_get_feed_by_id() {
        let db = setup_db();
        let user_id = create_test_user(&db);

        let new_feed = NewRssFeed::new("https://example.com/feed.xml", "Test Feed", user_id);
        let created = RssFeedRepository::create(db.conn(), &new_feed).unwrap();

        let retrieved = RssFeedRepository::get_by_id(db.conn(), created.id)
            .unwrap()
            .unwrap();
        assert_eq!(retrieved.id, created.id);
        assert_eq!(retrieved.title, "Test Feed");
    }

    #[test]
    fn test_get_feed_by_url() {
        let db = setup_db();
        let user_id = create_test_user(&db);

        let url = "https://example.com/feed.xml";
        let new_feed = NewRssFeed::new(url, "Test Feed", user_id);
        RssFeedRepository::create(db.conn(), &new_feed).unwrap();

        let retrieved = RssFeedRepository::get_by_url(db.conn(), url)
            .unwrap()
            .unwrap();
        assert_eq!(retrieved.url, url);
    }

    #[test]
    fn test_list_active_feeds() {
        let db = setup_db();
        let user_id = create_test_user(&db);

        // Create active and inactive feeds
        let feed1 = NewRssFeed::new("https://example1.com/feed.xml", "Feed 1", user_id);
        let feed2 = NewRssFeed::new("https://example2.com/feed.xml", "Feed 2", user_id);
        RssFeedRepository::create(db.conn(), &feed1).unwrap();
        let created2 = RssFeedRepository::create(db.conn(), &feed2).unwrap();

        // Disable feed2
        RssFeedRepository::update(db.conn(), created2.id, &RssFeedUpdate::new().disable()).unwrap();

        let active = RssFeedRepository::list_active(db.conn()).unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].title, "Feed 1");
    }

    #[test]
    fn test_update_feed() {
        let db = setup_db();
        let user_id = create_test_user(&db);

        let new_feed = NewRssFeed::new("https://example.com/feed.xml", "Test Feed", user_id);
        let feed = RssFeedRepository::create(db.conn(), &new_feed).unwrap();

        let update = RssFeedUpdate::new()
            .with_title("Updated Title")
            .with_fetch_interval(7200);
        RssFeedRepository::update(db.conn(), feed.id, &update).unwrap();

        let updated = RssFeedRepository::get_by_id(db.conn(), feed.id)
            .unwrap()
            .unwrap();
        assert_eq!(updated.title, "Updated Title");
        assert_eq!(updated.fetch_interval, 7200);
    }

    #[test]
    fn test_increment_and_clear_error() {
        let db = setup_db();
        let user_id = create_test_user(&db);

        let new_feed = NewRssFeed::new("https://example.com/feed.xml", "Test Feed", user_id);
        let feed = RssFeedRepository::create(db.conn(), &new_feed).unwrap();

        // Increment error
        RssFeedRepository::increment_error(db.conn(), feed.id, "Connection timeout").unwrap();
        let updated = RssFeedRepository::get_by_id(db.conn(), feed.id)
            .unwrap()
            .unwrap();
        assert_eq!(updated.error_count, 1);
        assert_eq!(updated.last_error, Some("Connection timeout".to_string()));

        // Clear error
        RssFeedRepository::clear_error(db.conn(), feed.id).unwrap();
        let cleared = RssFeedRepository::get_by_id(db.conn(), feed.id)
            .unwrap()
            .unwrap();
        assert_eq!(cleared.error_count, 0);
        assert!(cleared.last_error.is_none());
    }

    #[test]
    fn test_delete_feed() {
        let db = setup_db();
        let user_id = create_test_user(&db);

        let new_feed = NewRssFeed::new("https://example.com/feed.xml", "Test Feed", user_id);
        let feed = RssFeedRepository::create(db.conn(), &new_feed).unwrap();

        RssFeedRepository::delete(db.conn(), feed.id).unwrap();

        let result = RssFeedRepository::get_by_id(db.conn(), feed.id).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_create_item() {
        let db = setup_db();
        let user_id = create_test_user(&db);

        let new_feed = NewRssFeed::new("https://example.com/feed.xml", "Test Feed", user_id);
        let feed = RssFeedRepository::create(db.conn(), &new_feed).unwrap();

        let new_item = NewRssItem::new(feed.id, "guid-123", "Test Article")
            .with_link("https://example.com/article")
            .with_description("Article summary");

        let item_id = RssItemRepository::create_or_ignore(db.conn(), &new_item)
            .unwrap()
            .unwrap();

        let item = RssItemRepository::get_by_id(db.conn(), item_id)
            .unwrap()
            .unwrap();
        assert_eq!(item.guid, "guid-123");
        assert_eq!(item.title, "Test Article");
        assert_eq!(item.link, Some("https://example.com/article".to_string()));
    }

    #[test]
    fn test_create_item_ignores_duplicate() {
        let db = setup_db();
        let user_id = create_test_user(&db);

        let new_feed = NewRssFeed::new("https://example.com/feed.xml", "Test Feed", user_id);
        let feed = RssFeedRepository::create(db.conn(), &new_feed).unwrap();

        let new_item = NewRssItem::new(feed.id, "guid-123", "Test Article");

        // First insert
        let id1 = RssItemRepository::create_or_ignore(db.conn(), &new_item).unwrap();
        assert!(id1.is_some());

        // Second insert (duplicate) should be ignored
        let id2 = RssItemRepository::create_or_ignore(db.conn(), &new_item).unwrap();
        assert!(id2.is_none());

        // Should still have only one item
        assert_eq!(
            RssItemRepository::count_by_feed(db.conn(), feed.id).unwrap(),
            1
        );
    }

    #[test]
    fn test_list_items_by_feed() {
        let db = setup_db();
        let user_id = create_test_user(&db);

        let new_feed = NewRssFeed::new("https://example.com/feed.xml", "Test Feed", user_id);
        let feed = RssFeedRepository::create(db.conn(), &new_feed).unwrap();

        // Create items
        for i in 1..=5 {
            let item = NewRssItem::new(feed.id, format!("guid-{}", i), format!("Article {}", i));
            RssItemRepository::create_or_ignore(db.conn(), &item).unwrap();
        }

        let items = RssItemRepository::list_by_feed(db.conn(), feed.id, 3, 0).unwrap();
        assert_eq!(items.len(), 3);

        let items_page2 = RssItemRepository::list_by_feed(db.conn(), feed.id, 3, 3).unwrap();
        assert_eq!(items_page2.len(), 2);
    }

    #[test]
    fn test_prune_old_items() {
        let db = setup_db();
        let user_id = create_test_user(&db);

        let new_feed = NewRssFeed::new("https://example.com/feed.xml", "Test Feed", user_id);
        let feed = RssFeedRepository::create(db.conn(), &new_feed).unwrap();

        // Create more items than MAX_ITEMS_PER_FEED
        for i in 1..=150 {
            let item = NewRssItem::new(feed.id, format!("guid-{}", i), format!("Article {}", i));
            RssItemRepository::create_or_ignore(db.conn(), &item).unwrap();
        }

        assert_eq!(
            RssItemRepository::count_by_feed(db.conn(), feed.id).unwrap(),
            150
        );

        RssItemRepository::prune_old_items(db.conn(), feed.id).unwrap();

        assert_eq!(
            RssItemRepository::count_by_feed(db.conn(), feed.id).unwrap(),
            MAX_ITEMS_PER_FEED as i64
        );
    }

    #[test]
    fn test_read_position_upsert() {
        let db = setup_db();
        let user_id = create_test_user(&db);

        let new_feed = NewRssFeed::new("https://example.com/feed.xml", "Test Feed", user_id);
        let feed = RssFeedRepository::create(db.conn(), &new_feed).unwrap();

        let item = NewRssItem::new(feed.id, "guid-1", "Article 1");
        let item_id = RssItemRepository::create_or_ignore(db.conn(), &item)
            .unwrap()
            .unwrap();

        // Insert
        RssReadPositionRepository::upsert(db.conn(), user_id, feed.id, item_id).unwrap();

        let pos = RssReadPositionRepository::get(db.conn(), user_id, feed.id)
            .unwrap()
            .unwrap();
        assert_eq!(pos.last_read_item_id, Some(item_id));

        // Update
        let item2 = NewRssItem::new(feed.id, "guid-2", "Article 2");
        let item_id2 = RssItemRepository::create_or_ignore(db.conn(), &item2)
            .unwrap()
            .unwrap();

        RssReadPositionRepository::upsert(db.conn(), user_id, feed.id, item_id2).unwrap();

        let pos2 = RssReadPositionRepository::get(db.conn(), user_id, feed.id)
            .unwrap()
            .unwrap();
        assert_eq!(pos2.last_read_item_id, Some(item_id2));
    }

    #[test]
    fn test_count_unread() {
        let db = setup_db();
        let user_id = create_test_user(&db);

        let new_feed = NewRssFeed::new("https://example.com/feed.xml", "Test Feed", user_id);
        let feed = RssFeedRepository::create(db.conn(), &new_feed).unwrap();

        // Create 5 items
        for i in 1..=5 {
            let item = NewRssItem::new(feed.id, format!("guid-{}", i), format!("Article {}", i));
            RssItemRepository::create_or_ignore(db.conn(), &item).unwrap();
        }

        // All should be unread
        assert_eq!(
            RssItemRepository::count_unread(db.conn(), feed.id, user_id).unwrap(),
            5
        );

        // Mark item 3 as read
        let item3 = RssItemRepository::get_by_guid(db.conn(), feed.id, "guid-3")
            .unwrap()
            .unwrap();
        RssReadPositionRepository::upsert(db.conn(), user_id, feed.id, item3.id).unwrap();

        // Items 4, 5 should be unread
        assert_eq!(
            RssItemRepository::count_unread(db.conn(), feed.id, user_id).unwrap(),
            2
        );
    }

    #[test]
    fn test_mark_all_as_read() {
        let db = setup_db();
        let user_id = create_test_user(&db);

        let new_feed = NewRssFeed::new("https://example.com/feed.xml", "Test Feed", user_id);
        let feed = RssFeedRepository::create(db.conn(), &new_feed).unwrap();

        // Create items
        for i in 1..=5 {
            let item = NewRssItem::new(feed.id, format!("guid-{}", i), format!("Article {}", i));
            RssItemRepository::create_or_ignore(db.conn(), &item).unwrap();
        }

        RssReadPositionRepository::mark_all_as_read(db.conn(), user_id, feed.id).unwrap();

        assert_eq!(
            RssItemRepository::count_unread(db.conn(), feed.id, user_id).unwrap(),
            0
        );
    }

    #[test]
    fn test_list_with_unread() {
        let db = setup_db();
        let user_id = create_test_user(&db);

        let new_feed = NewRssFeed::new("https://example.com/feed.xml", "Test Feed", user_id);
        let feed = RssFeedRepository::create(db.conn(), &new_feed).unwrap();

        // Create items
        for i in 1..=3 {
            let item = NewRssItem::new(feed.id, format!("guid-{}", i), format!("Article {}", i));
            RssItemRepository::create_or_ignore(db.conn(), &item).unwrap();
        }

        let feeds_with_unread =
            RssFeedRepository::list_with_unread(db.conn(), Some(user_id)).unwrap();
        assert_eq!(feeds_with_unread.len(), 1);
        assert_eq!(feeds_with_unread[0].unread_count, 3);
    }
}
