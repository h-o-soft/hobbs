//! RSS handlers for Web API.

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;
use std::sync::Arc;
use utoipa::ToSchema;
use validator::Validate;

use crate::rss::{NewRssFeed, RssFeedRepository, RssItemRepository, RssReadPositionRepository};
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

/// Request to add a new RSS feed.
#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct AddFeedRequest {
    /// Feed URL.
    #[validate(url(message = "Invalid URL format"))]
    #[validate(length(min = 1, max = 2048, message = "URL must be 1-2048 characters"))]
    pub url: String,
    /// Feed title (optional, will use URL if not provided).
    #[serde(default)]
    #[validate(length(max = 100, message = "Title must be 100 characters or less"))]
    pub title: Option<String>,
}

/// GET /api/rss - List user's RSS feeds.
pub async fn list_feeds(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser,
) -> Result<Json<ApiResponse<Vec<RssFeedWithUnreadResponse>>>, ApiError> {
    let feed_repo = RssFeedRepository::new(state.db.pool());
    let feeds = feed_repo
        .list_with_unread(Some(claims.sub))
        .await
        .map_err(|e| {
            tracing::error!("Failed to list RSS feeds: {}", e);
            ApiError::internal("Failed to list RSS feeds")
        })?;

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

/// POST /api/rss/feeds - Add a new RSS feed.
pub async fn add_feed(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser,
    Json(req): Json<AddFeedRequest>,
) -> Result<Json<ApiResponse<RssFeedResponse>>, ApiError> {
    req.validate().map_err(ApiError::from_validation_errors)?;

    let title = req.title.unwrap_or_else(|| req.url.clone());
    let new_feed = NewRssFeed::new(&req.url, title, claims.sub);

    let feed_repo = RssFeedRepository::new(state.db.pool());

    // Check if user already has this feed
    if feed_repo
        .get_by_user_url(claims.sub, &req.url)
        .await
        .map_err(|e| {
            tracing::error!("Failed to check existing feed: {}", e);
            ApiError::internal("Failed to check existing feed")
        })?
        .is_some()
    {
        return Err(ApiError::conflict("Feed already exists"));
    }

    let feed = feed_repo
        .create(&new_feed)
        .await
        .map_err(|e| {
            tracing::error!("Failed to add RSS feed: {}", e);
            ApiError::internal("Failed to add RSS feed")
        })?;

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

/// DELETE /api/rss/feeds/:id - Delete a user's RSS feed.
pub async fn delete_feed(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser,
    Path(feed_id): Path<i64>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    let feed_repo = RssFeedRepository::new(state.db.pool());

    // Check if feed exists and belongs to user
    let feed = feed_repo
        .get_by_id(feed_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get RSS feed: {}", e);
            ApiError::internal("Failed to get RSS feed")
        })?
        .ok_or_else(|| ApiError::not_found("Feed not found"))?;

    // Only allow deleting own feeds
    if feed.created_by != claims.sub {
        return Err(ApiError::forbidden(
            "Cannot delete feed owned by another user",
        ));
    }

    feed_repo
        .delete(feed_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to delete RSS feed: {}", e);
            ApiError::internal("Failed to delete RSS feed")
        })?;

    Ok(Json(ApiResponse::new(())))
}

/// GET /api/rss/:id - Get feed details.
pub async fn get_feed(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser,
    Path(feed_id): Path<i64>,
) -> Result<Json<ApiResponse<RssFeedResponse>>, ApiError> {
    let feed_repo = RssFeedRepository::new(state.db.pool());
    let feed = feed_repo
        .get_by_id(feed_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get RSS feed: {}", e);
            ApiError::internal("Failed to get RSS feed")
        })?
        .ok_or_else(|| ApiError::not_found("Feed not found"))?;

    // Only show feeds owned by user
    if feed.created_by != claims.sub {
        return Err(ApiError::not_found("Feed not found"));
    }

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
    AuthUser(claims): AuthUser,
    Path(feed_id): Path<i64>,
    Query(pagination): Query<PaginationQuery>,
) -> Result<Json<PaginatedResponse<RssItemResponse>>, ApiError> {
    let (offset, limit) = pagination.to_offset_limit();

    let feed_repo = RssFeedRepository::new(state.db.pool());
    let item_repo = RssItemRepository::new(state.db.pool());

    // Check if feed exists and belongs to user
    let feed = feed_repo
        .get_by_id(feed_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get RSS feed: {}", e);
            ApiError::internal("Failed to get RSS feed")
        })?
        .ok_or_else(|| ApiError::not_found("Feed not found"))?;

    // Only allow accessing own feeds
    if feed.created_by != claims.sub {
        return Err(ApiError::not_found("Feed not found"));
    }

    if !feed.is_active {
        return Err(ApiError::not_found("Feed not found"));
    }

    let total = item_repo
        .count_by_feed(feed_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to count RSS items: {}", e);
            ApiError::internal("Failed to count RSS items")
        })?;

    let items = item_repo
        .list_by_feed(feed_id, limit as usize, offset as usize)
        .await
        .map_err(|e| {
            tracing::error!("Failed to list RSS items: {}", e);
            ApiError::internal("Failed to list RSS items")
        })?;

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
    AuthUser(claims): AuthUser,
    Path((feed_id, item_id)): Path<(i64, i64)>,
) -> Result<Json<ApiResponse<RssItemResponse>>, ApiError> {
    let feed_repo = RssFeedRepository::new(state.db.pool());
    let item_repo = RssItemRepository::new(state.db.pool());

    // Check if feed exists and belongs to user
    let feed = feed_repo
        .get_by_id(feed_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get RSS feed: {}", e);
            ApiError::internal("Failed to get RSS feed")
        })?
        .ok_or_else(|| ApiError::not_found("Feed not found"))?;

    // Only allow accessing own feeds
    if feed.created_by != claims.sub {
        return Err(ApiError::not_found("Feed not found"));
    }

    if !feed.is_active {
        return Err(ApiError::not_found("Feed not found"));
    }

    let item = item_repo
        .get_by_id(item_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get RSS item: {}", e);
            ApiError::internal("Failed to get RSS item")
        })?
        .ok_or_else(|| ApiError::not_found("Item not found"))?;

    // Verify item belongs to the feed
    if item.feed_id != feed_id {
        return Err(ApiError::not_found("Item not found"));
    }

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
    let feed_repo = RssFeedRepository::new(state.db.pool());
    let read_pos_repo = RssReadPositionRepository::new(state.db.pool());

    // Check if feed exists and belongs to user
    let feed = feed_repo
        .get_by_id(feed_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get RSS feed: {}", e);
            ApiError::internal("Failed to get RSS feed")
        })?
        .ok_or_else(|| ApiError::not_found("Feed not found"))?;

    // Only allow marking own feeds
    if feed.created_by != claims.sub {
        return Err(ApiError::not_found("Feed not found"));
    }

    if !feed.is_active {
        return Err(ApiError::not_found("Feed not found"));
    }

    read_pos_repo
        .mark_all_as_read(claims.sub, feed_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to mark RSS items as read: {}", e);
            ApiError::internal("Failed to mark items as read")
        })?;

    Ok(Json(ApiResponse::new(())))
}
