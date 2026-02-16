//! RSS repositories for HOBBS.

use chrono::{DateTime, Utc};
#[cfg(feature = "postgres")]
use sqlx::QueryBuilder;
#[cfg(feature = "sqlite")]
use sqlx::QueryBuilder;

use super::types::{
    NewRssFeed, NewRssItem, RssFeed, RssFeedUpdate, RssFeedWithUnread, RssItem, RssReadPosition,
    MAX_ITEMS_PER_FEED,
};
use crate::db::{DbPool, SQL_FALSE, SQL_TRUE};
use crate::{HobbsError, Result};

// SQL datetime function for current timestamp
#[cfg(feature = "sqlite")]
const SQL_NOW: &str = "datetime('now')";
#[cfg(feature = "postgres")]
const SQL_NOW: &str = "NOW()";

/// Row type for RSS feed from database.
#[derive(Debug, Clone, sqlx::FromRow)]
struct RssFeedRow {
    id: i64,
    url: String,
    title: String,
    description: Option<String>,
    site_url: Option<String>,
    last_fetched_at: Option<String>,
    last_item_at: Option<String>,
    fetch_interval: i64,
    is_active: bool,
    error_count: i32,
    last_error: Option<String>,
    created_by: i64,
    created_at: String,
    updated_at: String,
}

impl From<RssFeedRow> for RssFeed {
    fn from(row: RssFeedRow) -> Self {
        RssFeed {
            id: row.id,
            url: row.url,
            title: row.title,
            description: row.description,
            site_url: row.site_url,
            last_fetched_at: row.last_fetched_at.and_then(|s| parse_datetime(&s)),
            last_item_at: row.last_item_at.and_then(|s| parse_datetime(&s)),
            fetch_interval: row.fetch_interval,
            is_active: row.is_active,
            error_count: row.error_count,
            last_error: row.last_error,
            created_by: row.created_by,
            created_at: parse_datetime(&row.created_at).unwrap_or_else(Utc::now),
            updated_at: parse_datetime(&row.updated_at).unwrap_or_else(Utc::now),
        }
    }
}

/// Row type for RSS feed with unread count.
#[derive(Debug, Clone, sqlx::FromRow)]
struct RssFeedWithUnreadRow {
    id: i64,
    url: String,
    title: String,
    description: Option<String>,
    site_url: Option<String>,
    last_fetched_at: Option<String>,
    last_item_at: Option<String>,
    fetch_interval: i64,
    is_active: bool,
    error_count: i32,
    last_error: Option<String>,
    created_by: i64,
    created_at: String,
    updated_at: String,
    unread_count: i64,
}

impl From<RssFeedWithUnreadRow> for RssFeedWithUnread {
    fn from(row: RssFeedWithUnreadRow) -> Self {
        let feed = RssFeed {
            id: row.id,
            url: row.url,
            title: row.title,
            description: row.description,
            site_url: row.site_url,
            last_fetched_at: row.last_fetched_at.and_then(|s| parse_datetime(&s)),
            last_item_at: row.last_item_at.and_then(|s| parse_datetime(&s)),
            fetch_interval: row.fetch_interval,
            is_active: row.is_active,
            error_count: row.error_count,
            last_error: row.last_error,
            created_by: row.created_by,
            created_at: parse_datetime(&row.created_at).unwrap_or_else(Utc::now),
            updated_at: parse_datetime(&row.updated_at).unwrap_or_else(Utc::now),
        };
        RssFeedWithUnread {
            feed,
            unread_count: row.unread_count,
        }
    }
}

/// Row type for RSS item from database.
#[derive(Debug, Clone, sqlx::FromRow)]
struct RssItemRow {
    id: i64,
    feed_id: i64,
    guid: String,
    title: String,
    link: Option<String>,
    description: Option<String>,
    author: Option<String>,
    published_at: Option<String>,
    fetched_at: String,
}

impl From<RssItemRow> for RssItem {
    fn from(row: RssItemRow) -> Self {
        RssItem {
            id: row.id,
            feed_id: row.feed_id,
            guid: row.guid,
            title: row.title,
            link: row.link,
            description: row.description,
            author: row.author,
            published_at: row.published_at.and_then(|s| parse_datetime(&s)),
            fetched_at: parse_datetime(&row.fetched_at).unwrap_or_else(Utc::now),
        }
    }
}

/// Row type for RSS read position from database.
#[derive(Debug, Clone, sqlx::FromRow)]
struct RssReadPositionRow {
    id: i64,
    user_id: i64,
    feed_id: i64,
    last_read_item_id: Option<i64>,
    last_read_at: String,
}

impl From<RssReadPositionRow> for RssReadPosition {
    fn from(row: RssReadPositionRow) -> Self {
        RssReadPosition {
            id: row.id,
            user_id: row.user_id,
            feed_id: row.feed_id,
            last_read_item_id: row.last_read_item_id,
            last_read_at: parse_datetime(&row.last_read_at).unwrap_or_else(Utc::now),
        }
    }
}

/// Repository for RSS feed operations.
pub struct RssFeedRepository<'a> {
    pool: &'a DbPool,
}

impl<'a> RssFeedRepository<'a> {
    /// Create a new repository instance.
    pub fn new(pool: &'a DbPool) -> Self {
        Self { pool }
    }

    /// Create a new feed.
    pub async fn create(&self, feed: &NewRssFeed) -> Result<RssFeed> {
        let id: i64 = sqlx::query_scalar(
            r#"
            INSERT INTO rss_feeds (url, title, description, site_url, created_by)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id
            "#,
        )
        .bind(&feed.url)
        .bind(&feed.title)
        .bind(&feed.description)
        .bind(&feed.site_url)
        .bind(feed.created_by)
        .fetch_one(self.pool)
        .await
        .map_err(|e| HobbsError::Database(e.to_string()))?;

        self.get_by_id(id)
            .await?
            .ok_or_else(|| HobbsError::NotFound("RSS feed not found".into()))
    }

    /// Get a feed by ID.
    pub async fn get_by_id(&self, id: i64) -> Result<Option<RssFeed>> {
        let row = sqlx::query_as::<_, RssFeedRow>(
            r#"
            SELECT id, url, title, description, site_url, last_fetched_at, last_item_at,
                   fetch_interval, is_active, error_count, last_error, created_by,
                   created_at, updated_at
            FROM rss_feeds
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(self.pool)
        .await
        .map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(row.map(RssFeed::from))
    }

    /// Get a feed by URL (any user).
    pub async fn get_by_url(&self, url: &str) -> Result<Option<RssFeed>> {
        let row = sqlx::query_as::<_, RssFeedRow>(
            r#"
            SELECT id, url, title, description, site_url, last_fetched_at, last_item_at,
                   fetch_interval, is_active, error_count, last_error, created_by,
                   created_at, updated_at
            FROM rss_feeds
            WHERE url = $1
            "#,
        )
        .bind(url)
        .fetch_optional(self.pool)
        .await
        .map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(row.map(RssFeed::from))
    }

    /// Get a feed by URL for a specific user.
    pub async fn get_by_user_url(&self, user_id: i64, url: &str) -> Result<Option<RssFeed>> {
        let row = sqlx::query_as::<_, RssFeedRow>(
            r#"
            SELECT id, url, title, description, site_url, last_fetched_at, last_item_at,
                   fetch_interval, is_active, error_count, last_error, created_by,
                   created_at, updated_at
            FROM rss_feeds
            WHERE created_by = $1 AND url = $2
            "#,
        )
        .bind(user_id)
        .bind(url)
        .fetch_optional(self.pool)
        .await
        .map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(row.map(RssFeed::from))
    }

    /// List all active feeds (ordered by registration order).
    pub async fn list_active(&self) -> Result<Vec<RssFeed>> {
        let query = format!(
            r#"
            SELECT id, url, title, description, site_url, last_fetched_at, last_item_at,
                   fetch_interval, is_active, error_count, last_error, created_by,
                   created_at, updated_at
            FROM rss_feeds
            WHERE is_active = {}
            ORDER BY id ASC
            "#,
            SQL_TRUE
        );
        let rows = sqlx::query_as::<_, RssFeedRow>(&query)
            .fetch_all(self.pool)
            .await
            .map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(rows.into_iter().map(RssFeed::from).collect())
    }

    /// List active feeds for a specific user (ordered by registration order).
    pub async fn list_active_by_user(&self, user_id: i64) -> Result<Vec<RssFeed>> {
        let query = format!(
            r#"
            SELECT id, url, title, description, site_url, last_fetched_at, last_item_at,
                   fetch_interval, is_active, error_count, last_error, created_by,
                   created_at, updated_at
            FROM rss_feeds
            WHERE is_active = {} AND created_by = $1
            ORDER BY id ASC
            "#,
            SQL_TRUE
        );
        let rows = sqlx::query_as::<_, RssFeedRow>(&query)
            .bind(user_id)
            .fetch_all(self.pool)
            .await
            .map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(rows.into_iter().map(RssFeed::from).collect())
    }

    /// List all feeds (including inactive, ordered by registration order).
    pub async fn list_all(&self) -> Result<Vec<RssFeed>> {
        let rows = sqlx::query_as::<_, RssFeedRow>(
            r#"
            SELECT id, url, title, description, site_url, last_fetched_at, last_item_at,
                   fetch_interval, is_active, error_count, last_error, created_by,
                   created_at, updated_at
            FROM rss_feeds
            ORDER BY id ASC
            "#,
        )
        .fetch_all(self.pool)
        .await
        .map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(rows.into_iter().map(RssFeed::from).collect())
    }

    /// List feeds that are due for fetching.
    pub async fn list_due_for_fetch(&self) -> Result<Vec<RssFeed>> {
        #[cfg(feature = "sqlite")]
        let query = format!(
            r#"
            SELECT id, url, title, description, site_url, last_fetched_at, last_item_at,
                   fetch_interval, is_active, error_count, last_error, created_by,
                   created_at, updated_at
            FROM rss_feeds
            WHERE is_active = {}
              AND (last_fetched_at IS NULL
                   OR datetime(last_fetched_at, '+' || fetch_interval || ' seconds') <= datetime('now'))
            ORDER BY last_fetched_at ASC NULLS FIRST
            "#,
            SQL_TRUE
        );
        #[cfg(feature = "postgres")]
        let query = format!(
            r#"
            SELECT id, url, title, description, site_url, last_fetched_at, last_item_at,
                   fetch_interval, is_active, error_count, last_error, created_by,
                   created_at, updated_at
            FROM rss_feeds
            WHERE is_active = {}
              AND (last_fetched_at IS NULL
                   OR last_fetched_at + make_interval(secs => fetch_interval::double precision) <= NOW())
            ORDER BY last_fetched_at ASC NULLS FIRST
            "#,
            SQL_TRUE
        );
        let rows = sqlx::query_as::<_, RssFeedRow>(&query)
            .fetch_all(self.pool)
            .await
            .map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(rows.into_iter().map(RssFeed::from).collect())
    }

    /// List active feeds with unread counts for a user (ordered by registration order).
    /// Only returns feeds owned by the user (personal RSS reader).
    pub async fn list_with_unread(&self, user_id: Option<i64>) -> Result<Vec<RssFeedWithUnread>> {
        // If no user_id, return empty list (guest cannot have feeds)
        let user_id = match user_id {
            Some(id) => id,
            None => return Ok(Vec::new()),
        };

        let query = format!(
            r#"
            SELECT f.id, f.url, f.title, f.description, f.site_url, f.last_fetched_at, f.last_item_at,
                   f.fetch_interval, f.is_active, f.error_count, f.last_error, f.created_by,
                   f.created_at, f.updated_at,
                   (SELECT COUNT(*) FROM rss_items i
                    WHERE i.feed_id = f.id
                    AND i.id > COALESCE(
                        (SELECT last_read_item_id FROM rss_read_positions
                         WHERE user_id = $1 AND feed_id = f.id),
                        0)) as unread_count
            FROM rss_feeds f
            WHERE f.is_active = {} AND f.created_by = $2
            ORDER BY f.id ASC
            "#,
            SQL_TRUE
        );
        let rows = sqlx::query_as::<_, RssFeedWithUnreadRow>(&query)
            .bind(user_id)
            .bind(user_id)
            .fetch_all(self.pool)
            .await
            .map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(rows.into_iter().map(RssFeedWithUnread::from).collect())
    }

    /// Update a feed.
    #[cfg(feature = "sqlite")]
    pub async fn update(&self, id: i64, update: &RssFeedUpdate) -> Result<bool> {
        if update.is_empty() {
            return Ok(false);
        }

        let mut query: QueryBuilder<sqlx::Sqlite> = QueryBuilder::new("UPDATE rss_feeds SET ");
        let mut separated = query.separated(", ");

        if let Some(ref title) = update.title {
            separated.push("title = ");
            separated.push_bind_unseparated(title);
        }

        if let Some(ref description) = update.description {
            separated.push("description = ");
            separated.push_bind_unseparated(description.clone());
        }

        if let Some(interval) = update.fetch_interval {
            separated.push("fetch_interval = ");
            separated.push_bind_unseparated(interval);
        }

        if let Some(is_active) = update.is_active {
            separated.push("is_active = ");
            separated.push_bind_unseparated(is_active);
        }

        separated.push(format!("updated_at = {}", SQL_NOW));

        query.push(" WHERE id = ");
        query.push_bind(id);

        let result = query
            .build()
            .execute(self.pool)
            .await
            .map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(result.rows_affected() > 0)
    }

    /// Update a feed.
    #[cfg(feature = "postgres")]
    pub async fn update(&self, id: i64, update: &RssFeedUpdate) -> Result<bool> {
        if update.is_empty() {
            return Ok(false);
        }

        let mut query: QueryBuilder<sqlx::Postgres> = QueryBuilder::new("UPDATE rss_feeds SET ");
        let mut separated = query.separated(", ");

        if let Some(ref title) = update.title {
            separated.push("title = ");
            separated.push_bind_unseparated(title);
        }

        if let Some(ref description) = update.description {
            separated.push("description = ");
            separated.push_bind_unseparated(description.clone());
        }

        if let Some(interval) = update.fetch_interval {
            separated.push("fetch_interval = ");
            separated.push_bind_unseparated(interval);
        }

        if let Some(is_active) = update.is_active {
            separated.push("is_active = ");
            separated.push_bind_unseparated(is_active);
        }

        separated.push("updated_at = NOW()");

        query.push(" WHERE id = ");
        query.push_bind(id);

        let result = query
            .build()
            .execute(self.pool)
            .await
            .map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(result.rows_affected() > 0)
    }

    /// Update last fetched timestamp.
    pub async fn update_last_fetched(&self, id: i64) -> Result<bool> {
        let query = format!(
            "UPDATE rss_feeds SET last_fetched_at = {}, updated_at = {} WHERE id = $1",
            SQL_NOW, SQL_NOW
        );
        let result = sqlx::query(&query)
            .bind(id)
            .execute(self.pool)
            .await
            .map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(result.rows_affected() > 0)
    }

    /// Update last item timestamp.
    pub async fn update_last_item_at(&self, id: i64, last_item_at: DateTime<Utc>) -> Result<bool> {
        let query = format!(
            "UPDATE rss_feeds SET last_item_at = $1, updated_at = {} WHERE id = $2",
            SQL_NOW
        );
        let result = sqlx::query(&query)
            .bind(last_item_at.to_rfc3339())
            .bind(id)
            .execute(self.pool)
            .await
            .map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(result.rows_affected() > 0)
    }

    /// Increment error count and set error message.
    pub async fn increment_error(&self, id: i64, error: &str) -> Result<bool> {
        let query = format!(
            r#"
            UPDATE rss_feeds
            SET error_count = error_count + 1,
                last_error = $1,
                updated_at = {}
            WHERE id = $2
            "#,
            SQL_NOW
        );
        let result = sqlx::query(&query)
            .bind(error)
            .bind(id)
            .execute(self.pool)
            .await
            .map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(result.rows_affected() > 0)
    }

    /// Clear error count.
    pub async fn clear_error(&self, id: i64) -> Result<bool> {
        let query = format!(
            r#"
            UPDATE rss_feeds
            SET error_count = 0,
                last_error = NULL,
                updated_at = {}
            WHERE id = $1
            "#,
            SQL_NOW
        );
        let result = sqlx::query(&query)
            .bind(id)
            .execute(self.pool)
            .await
            .map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(result.rows_affected() > 0)
    }

    /// Disable feeds that have exceeded the error threshold.
    pub async fn disable_failed_feeds(&self, max_errors: i32) -> Result<u64> {
        let query = format!(
            r#"
            UPDATE rss_feeds
            SET is_active = {}, updated_at = {}
            WHERE error_count >= $1 AND is_active = {}
            "#,
            SQL_FALSE, SQL_NOW, SQL_TRUE
        );
        let result = sqlx::query(&query)
            .bind(max_errors)
            .execute(self.pool)
            .await
            .map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(result.rows_affected())
    }

    /// Delete a feed.
    pub async fn delete(&self, id: i64) -> Result<bool> {
        let result = sqlx::query("DELETE FROM rss_feeds WHERE id = $1")
            .bind(id)
            .execute(self.pool)
            .await
            .map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(result.rows_affected() > 0)
    }

    /// Count all feeds.
    pub async fn count(&self) -> Result<i64> {
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM rss_feeds")
            .fetch_one(self.pool)
            .await
            .map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(count.0)
    }
}

/// Repository for RSS item operations.
pub struct RssItemRepository<'a> {
    pool: &'a DbPool,
}

impl<'a> RssItemRepository<'a> {
    /// Create a new repository instance.
    pub fn new(pool: &'a DbPool) -> Self {
        Self { pool }
    }

    /// Create a new item, ignoring if duplicate (same feed_id + guid).
    #[cfg(feature = "sqlite")]
    pub async fn create_or_ignore(&self, item: &NewRssItem) -> Result<Option<i64>> {
        let published_at = item.published_at.map(|dt| dt.to_rfc3339());

        let result = sqlx::query(
            r#"
            INSERT OR IGNORE INTO rss_items (feed_id, guid, title, link, description, author, published_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(item.feed_id)
        .bind(&item.guid)
        .bind(&item.title)
        .bind(&item.link)
        .bind(&item.description)
        .bind(&item.author)
        .bind(&published_at)
        .execute(self.pool)
        .await
        .map_err(|e| HobbsError::Database(e.to_string()))?;

        if result.rows_affected() > 0 {
            Ok(Some(result.last_insert_rowid()))
        } else {
            Ok(None) // Already existed
        }
    }

    /// Create a new item, ignoring if duplicate (same feed_id + guid).
    #[cfg(feature = "postgres")]
    pub async fn create_or_ignore(&self, item: &NewRssItem) -> Result<Option<i64>> {
        let published_at = item.published_at.map(|dt| dt.to_rfc3339());

        let result: Option<(i64,)> = sqlx::query_as(
            r#"
            INSERT INTO rss_items (feed_id, guid, title, link, description, author, published_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (feed_id, guid) DO NOTHING
            RETURNING id
            "#,
        )
        .bind(item.feed_id)
        .bind(&item.guid)
        .bind(&item.title)
        .bind(&item.link)
        .bind(&item.description)
        .bind(&item.author)
        .bind(&published_at)
        .fetch_optional(self.pool)
        .await
        .map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(result.map(|(id,)| id))
    }

    /// Get an item by ID.
    pub async fn get_by_id(&self, id: i64) -> Result<Option<RssItem>> {
        let row = sqlx::query_as::<_, RssItemRow>(
            r#"
            SELECT id, feed_id, guid, title, link, description, author, published_at, fetched_at
            FROM rss_items
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(self.pool)
        .await
        .map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(row.map(RssItem::from))
    }

    /// Get an item by feed ID and guid.
    pub async fn get_by_guid(&self, feed_id: i64, guid: &str) -> Result<Option<RssItem>> {
        let row = sqlx::query_as::<_, RssItemRow>(
            r#"
            SELECT id, feed_id, guid, title, link, description, author, published_at, fetched_at
            FROM rss_items
            WHERE feed_id = $1 AND guid = $2
            "#,
        )
        .bind(feed_id)
        .bind(guid)
        .fetch_optional(self.pool)
        .await
        .map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(row.map(RssItem::from))
    }

    /// List items for a feed (newest first).
    pub async fn list_by_feed(
        &self,
        feed_id: i64,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<RssItem>> {
        let rows = sqlx::query_as::<_, RssItemRow>(
            r#"
            SELECT id, feed_id, guid, title, link, description, author, published_at, fetched_at
            FROM rss_items
            WHERE feed_id = $1
            ORDER BY COALESCE(published_at, fetched_at) DESC, id DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(feed_id)
        .bind(limit as i64)
        .bind(offset as i64)
        .fetch_all(self.pool)
        .await
        .map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(rows.into_iter().map(RssItem::from).collect())
    }

    /// Count items for a feed.
    pub async fn count_by_feed(&self, feed_id: i64) -> Result<i64> {
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM rss_items WHERE feed_id = $1")
            .bind(feed_id)
            .fetch_one(self.pool)
            .await
            .map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(count.0)
    }

    /// Count unread items for a user and feed.
    pub async fn count_unread(&self, feed_id: i64, user_id: i64) -> Result<i64> {
        let count: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM rss_items
            WHERE feed_id = $1
            AND id > COALESCE(
                (SELECT last_read_item_id FROM rss_read_positions
                 WHERE user_id = $2 AND feed_id = $3),
                0)
            "#,
        )
        .bind(feed_id)
        .bind(user_id)
        .bind(feed_id)
        .fetch_one(self.pool)
        .await
        .map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(count.0)
    }

    /// Get the next unread item for a user and feed (oldest unread by id).
    pub async fn get_next_unread(
        &self,
        feed_id: i64,
        user_id: i64,
    ) -> Result<Option<RssItem>> {
        let row = sqlx::query_as::<_, RssItemRow>(
            r#"
            SELECT id, feed_id, guid, title, link, description, author, published_at, fetched_at
            FROM rss_items
            WHERE feed_id = $1
            AND id > COALESCE(
                (SELECT last_read_item_id FROM rss_read_positions
                 WHERE user_id = $2 AND feed_id = $3),
                0)
            ORDER BY id ASC
            LIMIT 1
            "#,
        )
        .bind(feed_id)
        .bind(user_id)
        .bind(feed_id)
        .fetch_optional(self.pool)
        .await
        .map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(row.map(RssItem::from))
    }

    /// Get the newest item ID for a feed.
    pub async fn get_newest_item_id(&self, feed_id: i64) -> Result<Option<i64>> {
        let result: Option<(i64,)> = sqlx::query_as(
            r#"
            SELECT id FROM rss_items
            WHERE feed_id = $1
            ORDER BY COALESCE(published_at, fetched_at) DESC, id DESC
            LIMIT 1
            "#,
        )
        .bind(feed_id)
        .fetch_optional(self.pool)
        .await
        .map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(result.map(|r| r.0))
    }

    /// Delete old items for a feed, keeping only the most recent.
    pub async fn prune_old_items(&self, feed_id: i64) -> Result<u64> {
        let result = sqlx::query(
            r#"
            DELETE FROM rss_items
            WHERE feed_id = $1
            AND id NOT IN (
                SELECT id FROM rss_items
                WHERE feed_id = $2
                ORDER BY COALESCE(published_at, fetched_at) DESC, id DESC
                LIMIT $3
            )
            "#,
        )
        .bind(feed_id)
        .bind(feed_id)
        .bind(MAX_ITEMS_PER_FEED as i64)
        .execute(self.pool)
        .await
        .map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(result.rows_affected())
    }

    /// Delete all items for a feed.
    pub async fn delete_by_feed(&self, feed_id: i64) -> Result<u64> {
        let result = sqlx::query("DELETE FROM rss_items WHERE feed_id = $1")
            .bind(feed_id)
            .execute(self.pool)
            .await
            .map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(result.rows_affected())
    }
}

/// Repository for RSS read position operations.
pub struct RssReadPositionRepository<'a> {
    pool: &'a DbPool,
}

impl<'a> RssReadPositionRepository<'a> {
    /// Create a new repository instance.
    pub fn new(pool: &'a DbPool) -> Self {
        Self { pool }
    }

    /// Get read position for a user and feed.
    pub async fn get(&self, user_id: i64, feed_id: i64) -> Result<Option<RssReadPosition>> {
        let row = sqlx::query_as::<_, RssReadPositionRow>(
            r#"
            SELECT id, user_id, feed_id, last_read_item_id, last_read_at
            FROM rss_read_positions
            WHERE user_id = $1 AND feed_id = $2
            "#,
        )
        .bind(user_id)
        .bind(feed_id)
        .fetch_optional(self.pool)
        .await
        .map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(row.map(RssReadPosition::from))
    }

    /// Update or insert read position.
    pub async fn upsert(&self, user_id: i64, feed_id: i64, last_read_item_id: i64) -> Result<()> {
        let query = format!(
            r#"
            INSERT INTO rss_read_positions (user_id, feed_id, last_read_item_id, last_read_at)
            VALUES ($1, $2, $3, {})
            ON CONFLICT(user_id, feed_id) DO UPDATE SET
                last_read_item_id = $4,
                last_read_at = {}
            "#,
            SQL_NOW, SQL_NOW
        );
        sqlx::query(&query)
            .bind(user_id)
            .bind(feed_id)
            .bind(last_read_item_id)
            .bind(last_read_item_id)
            .execute(self.pool)
            .await
            .map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(())
    }

    /// Mark all items as read (set to newest item ID).
    pub async fn mark_all_as_read(&self, user_id: i64, feed_id: i64) -> Result<bool> {
        let item_repo = RssItemRepository::new(self.pool);
        let newest_id = item_repo.get_newest_item_id(feed_id).await?;
        match newest_id {
            Some(id) => {
                self.upsert(user_id, feed_id, id).await?;
                Ok(true)
            }
            None => Ok(false), // No items to mark as read
        }
    }

    /// Delete read position for a user and feed.
    pub async fn delete(&self, user_id: i64, feed_id: i64) -> Result<bool> {
        let result =
            sqlx::query("DELETE FROM rss_read_positions WHERE user_id = $1 AND feed_id = $2")
                .bind(user_id)
                .bind(feed_id)
                .execute(self.pool)
                .await
                .map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(result.rows_affected() > 0)
    }

    /// Delete all read positions for a user.
    pub async fn delete_by_user(&self, user_id: i64) -> Result<u64> {
        let result = sqlx::query("DELETE FROM rss_read_positions WHERE user_id = $1")
            .bind(user_id)
            .execute(self.pool)
            .await
            .map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(result.rows_affected())
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
    use crate::db::{NewUser, UserRepository};
    use crate::Database;

    async fn setup_db() -> Database {
        Database::open_in_memory().await.unwrap()
    }

    async fn create_test_user(db: &Database) -> i64 {
        let repo = UserRepository::new(db.pool());
        let user = NewUser::new("testuser", "password123", "Test User");
        repo.create(&user).await.unwrap().id
    }

    #[tokio::test]
    async fn test_create_feed() {
        let db = setup_db().await;
        let user_id = create_test_user(&db).await;
        let repo = RssFeedRepository::new(db.pool());

        let new_feed = NewRssFeed::new("https://example.com/feed.xml", "Test Feed", user_id);
        let feed = repo.create(&new_feed).await.unwrap();

        assert!(feed.id > 0);
        assert_eq!(feed.url, "https://example.com/feed.xml");
        assert_eq!(feed.title, "Test Feed");
        assert_eq!(feed.created_by, user_id);
        assert!(feed.is_active);
        assert_eq!(feed.error_count, 0);
    }

    #[tokio::test]
    async fn test_get_feed_by_id() {
        let db = setup_db().await;
        let user_id = create_test_user(&db).await;
        let repo = RssFeedRepository::new(db.pool());

        let new_feed = NewRssFeed::new("https://example.com/feed.xml", "Test Feed", user_id);
        let created = repo.create(&new_feed).await.unwrap();

        let retrieved = repo.get_by_id(created.id).await.unwrap().unwrap();
        assert_eq!(retrieved.id, created.id);
        assert_eq!(retrieved.title, "Test Feed");
    }

    #[tokio::test]
    async fn test_get_feed_by_url() {
        let db = setup_db().await;
        let user_id = create_test_user(&db).await;
        let repo = RssFeedRepository::new(db.pool());

        let url = "https://example.com/feed.xml";
        let new_feed = NewRssFeed::new(url, "Test Feed", user_id);
        repo.create(&new_feed).await.unwrap();

        let retrieved = repo.get_by_url(url).await.unwrap().unwrap();
        assert_eq!(retrieved.url, url);
    }

    #[tokio::test]
    async fn test_list_active_feeds() {
        let db = setup_db().await;
        let user_id = create_test_user(&db).await;
        let repo = RssFeedRepository::new(db.pool());

        // Create active and inactive feeds
        let feed1 = NewRssFeed::new("https://example1.com/feed.xml", "Feed 1", user_id);
        let feed2 = NewRssFeed::new("https://example2.com/feed.xml", "Feed 2", user_id);
        repo.create(&feed1).await.unwrap();
        let created2 = repo.create(&feed2).await.unwrap();

        // Disable feed2
        repo.update(created2.id, &RssFeedUpdate::new().disable())
            .await
            .unwrap();

        let active = repo.list_active().await.unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].title, "Feed 1");
    }

    #[tokio::test]
    async fn test_update_feed() {
        let db = setup_db().await;
        let user_id = create_test_user(&db).await;
        let repo = RssFeedRepository::new(db.pool());

        let new_feed = NewRssFeed::new("https://example.com/feed.xml", "Test Feed", user_id);
        let feed = repo.create(&new_feed).await.unwrap();

        let update = RssFeedUpdate::new()
            .with_title("Updated Title")
            .with_fetch_interval(7200);
        repo.update(feed.id, &update).await.unwrap();

        let updated = repo.get_by_id(feed.id).await.unwrap().unwrap();
        assert_eq!(updated.title, "Updated Title");
        assert_eq!(updated.fetch_interval, 7200);
    }

    #[tokio::test]
    async fn test_increment_and_clear_error() {
        let db = setup_db().await;
        let user_id = create_test_user(&db).await;
        let repo = RssFeedRepository::new(db.pool());

        let new_feed = NewRssFeed::new("https://example.com/feed.xml", "Test Feed", user_id);
        let feed = repo.create(&new_feed).await.unwrap();

        // Increment error
        repo.increment_error(feed.id, "Connection timeout")
            .await
            .unwrap();
        let updated = repo.get_by_id(feed.id).await.unwrap().unwrap();
        assert_eq!(updated.error_count, 1);
        assert_eq!(updated.last_error, Some("Connection timeout".to_string()));

        // Clear error
        repo.clear_error(feed.id).await.unwrap();
        let cleared = repo.get_by_id(feed.id).await.unwrap().unwrap();
        assert_eq!(cleared.error_count, 0);
        assert!(cleared.last_error.is_none());
    }

    #[tokio::test]
    async fn test_delete_feed() {
        let db = setup_db().await;
        let user_id = create_test_user(&db).await;
        let repo = RssFeedRepository::new(db.pool());

        let new_feed = NewRssFeed::new("https://example.com/feed.xml", "Test Feed", user_id);
        let feed = repo.create(&new_feed).await.unwrap();

        repo.delete(feed.id).await.unwrap();

        let result = repo.get_by_id(feed.id).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_create_item() {
        let db = setup_db().await;
        let user_id = create_test_user(&db).await;
        let feed_repo = RssFeedRepository::new(db.pool());
        let item_repo = RssItemRepository::new(db.pool());

        let new_feed = NewRssFeed::new("https://example.com/feed.xml", "Test Feed", user_id);
        let feed = feed_repo.create(&new_feed).await.unwrap();

        let new_item = NewRssItem::new(feed.id, "guid-123", "Test Article")
            .with_link("https://example.com/article")
            .with_description("Article summary");

        let item_id = item_repo
            .create_or_ignore(&new_item)
            .await
            .unwrap()
            .unwrap();

        let item = item_repo.get_by_id(item_id).await.unwrap().unwrap();
        assert_eq!(item.guid, "guid-123");
        assert_eq!(item.title, "Test Article");
        assert_eq!(item.link, Some("https://example.com/article".to_string()));
    }

    #[tokio::test]
    async fn test_create_item_ignores_duplicate() {
        let db = setup_db().await;
        let user_id = create_test_user(&db).await;
        let feed_repo = RssFeedRepository::new(db.pool());
        let item_repo = RssItemRepository::new(db.pool());

        let new_feed = NewRssFeed::new("https://example.com/feed.xml", "Test Feed", user_id);
        let feed = feed_repo.create(&new_feed).await.unwrap();

        let new_item = NewRssItem::new(feed.id, "guid-123", "Test Article");

        // First insert
        let id1 = item_repo.create_or_ignore(&new_item).await.unwrap();
        assert!(id1.is_some());

        // Second insert (duplicate) should be ignored
        let id2 = item_repo.create_or_ignore(&new_item).await.unwrap();
        assert!(id2.is_none());

        // Should still have only one item
        assert_eq!(item_repo.count_by_feed(feed.id).await.unwrap(), 1);
    }

    #[tokio::test]
    async fn test_list_items_by_feed() {
        let db = setup_db().await;
        let user_id = create_test_user(&db).await;
        let feed_repo = RssFeedRepository::new(db.pool());
        let item_repo = RssItemRepository::new(db.pool());

        let new_feed = NewRssFeed::new("https://example.com/feed.xml", "Test Feed", user_id);
        let feed = feed_repo.create(&new_feed).await.unwrap();

        // Create items
        for i in 1..=5 {
            let item = NewRssItem::new(feed.id, format!("guid-{}", i), format!("Article {}", i));
            item_repo.create_or_ignore(&item).await.unwrap();
        }

        let items = item_repo.list_by_feed(feed.id, 3, 0).await.unwrap();
        assert_eq!(items.len(), 3);

        let items_page2 = item_repo.list_by_feed(feed.id, 3, 3).await.unwrap();
        assert_eq!(items_page2.len(), 2);
    }

    #[tokio::test]
    async fn test_prune_old_items() {
        let db = setup_db().await;
        let user_id = create_test_user(&db).await;
        let feed_repo = RssFeedRepository::new(db.pool());
        let item_repo = RssItemRepository::new(db.pool());

        let new_feed = NewRssFeed::new("https://example.com/feed.xml", "Test Feed", user_id);
        let feed = feed_repo.create(&new_feed).await.unwrap();

        // Create more items than MAX_ITEMS_PER_FEED
        for i in 1..=150 {
            let item = NewRssItem::new(feed.id, format!("guid-{}", i), format!("Article {}", i));
            item_repo.create_or_ignore(&item).await.unwrap();
        }

        assert_eq!(item_repo.count_by_feed(feed.id).await.unwrap(), 150);

        item_repo.prune_old_items(feed.id).await.unwrap();

        assert_eq!(
            item_repo.count_by_feed(feed.id).await.unwrap(),
            MAX_ITEMS_PER_FEED as i64
        );
    }

    #[tokio::test]
    async fn test_read_position_upsert() {
        let db = setup_db().await;
        let user_id = create_test_user(&db).await;
        let feed_repo = RssFeedRepository::new(db.pool());
        let item_repo = RssItemRepository::new(db.pool());
        let pos_repo = RssReadPositionRepository::new(db.pool());

        let new_feed = NewRssFeed::new("https://example.com/feed.xml", "Test Feed", user_id);
        let feed = feed_repo.create(&new_feed).await.unwrap();

        let item = NewRssItem::new(feed.id, "guid-1", "Article 1");
        let item_id = item_repo.create_or_ignore(&item).await.unwrap().unwrap();

        // Insert
        pos_repo.upsert(user_id, feed.id, item_id).await.unwrap();

        let pos = pos_repo.get(user_id, feed.id).await.unwrap().unwrap();
        assert_eq!(pos.last_read_item_id, Some(item_id));

        // Update
        let item2 = NewRssItem::new(feed.id, "guid-2", "Article 2");
        let item_id2 = item_repo.create_or_ignore(&item2).await.unwrap().unwrap();

        pos_repo.upsert(user_id, feed.id, item_id2).await.unwrap();

        let pos2 = pos_repo.get(user_id, feed.id).await.unwrap().unwrap();
        assert_eq!(pos2.last_read_item_id, Some(item_id2));
    }

    #[tokio::test]
    async fn test_count_unread() {
        let db = setup_db().await;
        let user_id = create_test_user(&db).await;
        let feed_repo = RssFeedRepository::new(db.pool());
        let item_repo = RssItemRepository::new(db.pool());
        let pos_repo = RssReadPositionRepository::new(db.pool());

        let new_feed = NewRssFeed::new("https://example.com/feed.xml", "Test Feed", user_id);
        let feed = feed_repo.create(&new_feed).await.unwrap();

        // Create 5 items
        for i in 1..=5 {
            let item = NewRssItem::new(feed.id, format!("guid-{}", i), format!("Article {}", i));
            item_repo.create_or_ignore(&item).await.unwrap();
        }

        // All should be unread
        assert_eq!(item_repo.count_unread(feed.id, user_id).await.unwrap(), 5);

        // Mark item 3 as read
        let item3 = item_repo
            .get_by_guid(feed.id, "guid-3")
            .await
            .unwrap()
            .unwrap();
        pos_repo.upsert(user_id, feed.id, item3.id).await.unwrap();

        // Items 4, 5 should be unread
        assert_eq!(item_repo.count_unread(feed.id, user_id).await.unwrap(), 2);
    }

    #[tokio::test]
    async fn test_mark_all_as_read() {
        let db = setup_db().await;
        let user_id = create_test_user(&db).await;
        let feed_repo = RssFeedRepository::new(db.pool());
        let item_repo = RssItemRepository::new(db.pool());
        let pos_repo = RssReadPositionRepository::new(db.pool());

        let new_feed = NewRssFeed::new("https://example.com/feed.xml", "Test Feed", user_id);
        let feed = feed_repo.create(&new_feed).await.unwrap();

        // Create items
        for i in 1..=5 {
            let item = NewRssItem::new(feed.id, format!("guid-{}", i), format!("Article {}", i));
            item_repo.create_or_ignore(&item).await.unwrap();
        }

        pos_repo.mark_all_as_read(user_id, feed.id).await.unwrap();

        assert_eq!(item_repo.count_unread(feed.id, user_id).await.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_list_with_unread() {
        let db = setup_db().await;
        let user_id = create_test_user(&db).await;
        let feed_repo = RssFeedRepository::new(db.pool());
        let item_repo = RssItemRepository::new(db.pool());

        let new_feed = NewRssFeed::new("https://example.com/feed.xml", "Test Feed", user_id);
        let feed = feed_repo.create(&new_feed).await.unwrap();

        // Create items
        for i in 1..=3 {
            let item = NewRssItem::new(feed.id, format!("guid-{}", i), format!("Article {}", i));
            item_repo.create_or_ignore(&item).await.unwrap();
        }

        let feeds_with_unread = feed_repo.list_with_unread(Some(user_id)).await.unwrap();
        assert_eq!(feeds_with_unread.len(), 1);
        assert_eq!(feeds_with_unread[0].unread_count, 3);
    }
}
