//! RSS handlers for Web API.

use axum::{
    extract::{Path, Query, State},
    Json,
};
use std::sync::Arc;

use crate::rss::{RssFeedRepository, RssItemRepository, RssReadPositionRepository};
use crate::web::dto::{
    ApiResponse, PaginatedResponse, PaginationQuery, RssFeedResponse, RssItemResponse,
};
use crate::web::error::ApiError;
use crate::web::handlers::AppState;
use crate::web::middleware::AuthUser;

/// Response for RSS feed with unread count.
#[derive(Debug, serde::Serialize)]
pub struct RssFeedWithUnreadResponse {
    /// Feed info.
    #[serde(flatten)]
    pub feed: RssFeedResponse,
    /// Unread item count.
    pub unread_count: i64,
}

/// GET /api/rss - List all active RSS feeds.
pub async fn list_feeds(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser,
) -> Result<Json<ApiResponse<Vec<RssFeedWithUnreadResponse>>>, ApiError> {
    let feeds = {
        let db = state.db.lock().await;

        RssFeedRepository::list_with_unread(db.conn(), Some(claims.sub)).map_err(|e| {
            tracing::error!("Failed to list RSS feeds: {}", e);
            ApiError::internal("Failed to list RSS feeds")
        })?
    };

    let responses: Vec<_> = feeds
        .into_iter()
        .map(|f| RssFeedWithUnreadResponse {
            feed: RssFeedResponse {
                id: f.feed.id,
                url: f.feed.url,
                title: f.feed.title,
                description: f.feed.description,
                site_url: f.feed.site_url,
                last_fetched_at: f.feed.last_fetched_at.map(|dt| dt.to_rfc3339()),
                is_active: f.feed.is_active,
            },
            unread_count: f.unread_count,
        })
        .collect();

    Ok(Json(ApiResponse::new(responses)))
}

/// GET /api/rss/:id - Get feed details.
pub async fn get_feed(
    State(state): State<Arc<AppState>>,
    AuthUser(_claims): AuthUser,
    Path(feed_id): Path<i64>,
) -> Result<Json<ApiResponse<RssFeedResponse>>, ApiError> {
    let feed = {
        let db = state.db.lock().await;

        RssFeedRepository::get_by_id(db.conn(), feed_id)
            .map_err(|e| {
                tracing::error!("Failed to get RSS feed: {}", e);
                ApiError::internal("Failed to get RSS feed")
            })?
            .ok_or_else(|| ApiError::not_found("Feed not found"))?
    };

    // Only show active feeds
    if !feed.is_active {
        return Err(ApiError::not_found("Feed not found"));
    }

    let response = RssFeedResponse {
        id: feed.id,
        url: feed.url,
        title: feed.title,
        description: feed.description,
        site_url: feed.site_url,
        last_fetched_at: feed.last_fetched_at.map(|dt| dt.to_rfc3339()),
        is_active: feed.is_active,
    };

    Ok(Json(ApiResponse::new(response)))
}

/// GET /api/rss/:id/items - List items for a feed.
pub async fn list_items(
    State(state): State<Arc<AppState>>,
    AuthUser(_claims): AuthUser,
    Path(feed_id): Path<i64>,
    Query(pagination): Query<PaginationQuery>,
) -> Result<Json<PaginatedResponse<RssItemResponse>>, ApiError> {
    let (offset, limit) = pagination.to_offset_limit();

    let (items, total) = {
        let db = state.db.lock().await;

        // Check if feed exists and is active
        let feed = RssFeedRepository::get_by_id(db.conn(), feed_id)
            .map_err(|e| {
                tracing::error!("Failed to get RSS feed: {}", e);
                ApiError::internal("Failed to get RSS feed")
            })?
            .ok_or_else(|| ApiError::not_found("Feed not found"))?;

        if !feed.is_active {
            return Err(ApiError::not_found("Feed not found"));
        }

        let total = RssItemRepository::count_by_feed(db.conn(), feed_id).map_err(|e| {
            tracing::error!("Failed to count RSS items: {}", e);
            ApiError::internal("Failed to count RSS items")
        })?;

        let items =
            RssItemRepository::list_by_feed(db.conn(), feed_id, limit as usize, offset as usize)
                .map_err(|e| {
                    tracing::error!("Failed to list RSS items: {}", e);
                    ApiError::internal("Failed to list RSS items")
                })?;

        (items, total)
    };

    let responses: Vec<_> = items
        .into_iter()
        .map(|item| RssItemResponse {
            id: item.id,
            feed_id: item.feed_id,
            title: item.title,
            link: item.link,
            description: item.description,
            author: item.author,
            published_at: item.published_at.map(|dt| dt.to_rfc3339()),
        })
        .collect();

    Ok(Json(PaginatedResponse::new(
        responses,
        pagination.page,
        pagination.per_page,
        total as u64,
    )))
}

/// GET /api/rss/:feed_id/items/:item_id - Get item details.
pub async fn get_item(
    State(state): State<Arc<AppState>>,
    AuthUser(_claims): AuthUser,
    Path((feed_id, item_id)): Path<(i64, i64)>,
) -> Result<Json<ApiResponse<RssItemResponse>>, ApiError> {
    let item = {
        let db = state.db.lock().await;

        // Check if feed exists and is active
        let feed = RssFeedRepository::get_by_id(db.conn(), feed_id)
            .map_err(|e| {
                tracing::error!("Failed to get RSS feed: {}", e);
                ApiError::internal("Failed to get RSS feed")
            })?
            .ok_or_else(|| ApiError::not_found("Feed not found"))?;

        if !feed.is_active {
            return Err(ApiError::not_found("Feed not found"));
        }

        let item = RssItemRepository::get_by_id(db.conn(), item_id)
            .map_err(|e| {
                tracing::error!("Failed to get RSS item: {}", e);
                ApiError::internal("Failed to get RSS item")
            })?
            .ok_or_else(|| ApiError::not_found("Item not found"))?;

        // Verify item belongs to the feed
        if item.feed_id != feed_id {
            return Err(ApiError::not_found("Item not found"));
        }

        item
    };

    let response = RssItemResponse {
        id: item.id,
        feed_id: item.feed_id,
        title: item.title,
        link: item.link,
        description: item.description,
        author: item.author,
        published_at: item.published_at.map(|dt| dt.to_rfc3339()),
    };

    Ok(Json(ApiResponse::new(response)))
}

/// POST /api/rss/:id/mark-read - Mark all items as read.
pub async fn mark_as_read(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser,
    Path(feed_id): Path<i64>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    {
        let db = state.db.lock().await;

        // Check if feed exists and is active
        let feed = RssFeedRepository::get_by_id(db.conn(), feed_id)
            .map_err(|e| {
                tracing::error!("Failed to get RSS feed: {}", e);
                ApiError::internal("Failed to get RSS feed")
            })?
            .ok_or_else(|| ApiError::not_found("Feed not found"))?;

        if !feed.is_active {
            return Err(ApiError::not_found("Feed not found"));
        }

        RssReadPositionRepository::mark_all_as_read(db.conn(), claims.sub, feed_id).map_err(
            |e| {
                tracing::error!("Failed to mark RSS items as read: {}", e);
                ApiError::internal("Failed to mark items as read")
            },
        )?;
    }

    Ok(Json(ApiResponse::new(())))
}
