//! Board handlers for Web API.

use axum::{
    extract::{Path, Query, State},
    Json,
};
use std::sync::Arc;

use crate::board::{
    BoardRepository, BoardType, NewFlatPost, NewThread, NewThreadPost, PostRepository,
    ThreadRepository,
};
use crate::db::{Role, UserRepository};
use crate::web::dto::{
    ApiResponse, AuthorInfo, BoardResponse, CreateFlatPostRequest, CreatePostRequest,
    CreateThreadRequest, PaginatedResponse, PaginationQuery, PostResponse, ThreadResponse,
};
use crate::web::error::ApiError;
use crate::web::handlers::AppState;
use crate::web::middleware::{AuthUser, OptionalAuthUser};

/// GET /api/boards - List all accessible boards.
pub async fn list_boards(
    State(state): State<Arc<AppState>>,
    OptionalAuthUser(auth): OptionalAuthUser,
) -> Result<Json<ApiResponse<Vec<BoardResponse>>>, ApiError> {
    let user_role = auth
        .map(|c| Role::from_str(&c.role).unwrap_or(Role::Guest))
        .unwrap_or(Role::Guest);

    let boards = {
        let db = state.db.lock().await;
        let repo = BoardRepository::new(&*db);
        repo.list_accessible(user_role).map_err(|e| {
            tracing::error!("Failed to list boards: {}", e);
            ApiError::internal("Failed to list boards")
        })?
    };

    let responses: Vec<BoardResponse> = boards
        .into_iter()
        .map(|b| {
            let can_read = b.can_read(user_role);
            let can_write = b.can_write(user_role);
            BoardResponse {
                id: b.id,
                name: b.name,
                description: b.description,
                board_type: b.board_type.as_str().to_string(),
                can_read,
                can_write,
                created_at: b.created_at,
            }
        })
        .collect();

    Ok(Json(ApiResponse::new(responses)))
}

/// GET /api/boards/:id - Get board details.
pub async fn get_board(
    State(state): State<Arc<AppState>>,
    OptionalAuthUser(auth): OptionalAuthUser,
    Path(board_id): Path<i64>,
) -> Result<Json<ApiResponse<BoardResponse>>, ApiError> {
    let user_role = auth
        .map(|c| Role::from_str(&c.role).unwrap_or(Role::Guest))
        .unwrap_or(Role::Guest);

    let board = {
        let db = state.db.lock().await;
        let repo = BoardRepository::new(&*db);
        repo.get_by_id(board_id)
            .map_err(|e| {
                tracing::error!("Failed to get board: {}", e);
                ApiError::internal("Failed to get board")
            })?
            .ok_or_else(|| ApiError::not_found("Board not found"))?
    };

    if !board.can_read(user_role) {
        return Err(ApiError::forbidden("Access denied"));
    }

    let can_read = board.can_read(user_role);
    let can_write = board.can_write(user_role);
    let response = BoardResponse {
        id: board.id,
        name: board.name,
        description: board.description,
        board_type: board.board_type.as_str().to_string(),
        can_read,
        can_write,
        created_at: board.created_at,
    };

    Ok(Json(ApiResponse::new(response)))
}

/// GET /api/boards/:id/threads - List threads in a board.
pub async fn list_threads(
    State(state): State<Arc<AppState>>,
    OptionalAuthUser(auth): OptionalAuthUser,
    Path(board_id): Path<i64>,
    Query(pagination): Query<PaginationQuery>,
) -> Result<Json<PaginatedResponse<ThreadResponse>>, ApiError> {
    let user_role = auth
        .map(|c| Role::from_str(&c.role).unwrap_or(Role::Guest))
        .unwrap_or(Role::Guest);

    let (offset, limit) = pagination.to_offset_limit();

    let (_board, threads, total) = {
        let db = state.db.lock().await;
        let board_repo = BoardRepository::new(&*db);
        let thread_repo = ThreadRepository::new(&*db);

        let board = board_repo
            .get_by_id(board_id)
            .map_err(|e| {
                tracing::error!("Failed to get board: {}", e);
                ApiError::internal("Database error")
            })?
            .ok_or_else(|| ApiError::not_found("Board not found"))?;

        if !board.can_read(user_role) {
            return Err(ApiError::forbidden("Access denied"));
        }

        if board.board_type != BoardType::Thread {
            return Err(ApiError::bad_request("This board does not support threads"));
        }

        let threads = thread_repo
            .list_by_board_paginated(board_id, offset, limit)
            .map_err(|e| {
                tracing::error!("Failed to list threads: {}", e);
                ApiError::internal("Database error")
            })?;

        let total = thread_repo.count_by_board(board_id).map_err(|e| {
            tracing::error!("Failed to count threads: {}", e);
            ApiError::internal("Database error")
        })?;

        (board, threads, total)
    };

    // Get author info for each thread
    let responses = {
        let db = state.db.lock().await;
        let user_repo = UserRepository::new(&*db);

        threads
            .into_iter()
            .map(|t| {
                let author = user_repo
                    .get_by_id(t.author_id)
                    .ok()
                    .flatten()
                    .map(|u| AuthorInfo {
                        id: u.id,
                        username: u.username,
                        nickname: u.nickname,
                    })
                    .unwrap_or_else(|| AuthorInfo {
                        id: t.author_id,
                        username: "unknown".to_string(),
                        nickname: "Unknown".to_string(),
                    });

                ThreadResponse {
                    id: t.id,
                    board_id: t.board_id,
                    title: t.title,
                    author,
                    post_count: t.post_count,
                    created_at: t.created_at,
                    updated_at: t.updated_at,
                }
            })
            .collect()
    };

    Ok(Json(PaginatedResponse::new(
        responses,
        pagination.page,
        pagination.per_page,
        total as u64,
    )))
}

/// POST /api/boards/:id/threads - Create a new thread.
pub async fn create_thread(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser,
    Path(board_id): Path<i64>,
    Json(req): Json<CreateThreadRequest>,
) -> Result<Json<ApiResponse<ThreadResponse>>, ApiError> {
    let user_role = Role::from_str(&claims.role).unwrap_or(Role::Guest);

    // Validate input
    if req.title.trim().is_empty() {
        return Err(ApiError::bad_request("Title is required"));
    }
    if req.body.trim().is_empty() {
        return Err(ApiError::bad_request("Body is required"));
    }

    let (thread, author) = {
        let db = state.db.lock().await;
        let board_repo = BoardRepository::new(&*db);
        let thread_repo = ThreadRepository::new(&*db);
        let post_repo = PostRepository::new(&*db);
        let user_repo = UserRepository::new(&*db);

        // Check board access
        let board = board_repo
            .get_by_id(board_id)
            .map_err(|e| {
                tracing::error!("Failed to get board: {}", e);
                ApiError::internal("Database error")
            })?
            .ok_or_else(|| ApiError::not_found("Board not found"))?;

        if !board.can_write(user_role) {
            return Err(ApiError::forbidden("Write access denied"));
        }

        if board.board_type != BoardType::Thread {
            return Err(ApiError::bad_request("This board does not support threads"));
        }

        // Create thread
        let new_thread = NewThread::new(board_id, &req.title, claims.sub);
        let thread = thread_repo.create(&new_thread).map_err(|e| {
            tracing::error!("Failed to create thread: {}", e);
            ApiError::internal("Failed to create thread")
        })?;

        // Create first post
        let new_post = NewThreadPost::new(board_id, thread.id, claims.sub, &req.body);
        post_repo.create_thread_post(&new_post).map_err(|e| {
            tracing::error!("Failed to create post: {}", e);
            ApiError::internal("Failed to create post")
        })?;

        // Update thread post count
        let thread = thread_repo
            .touch_and_increment(thread.id)
            .map_err(|e| {
                tracing::error!("Failed to update thread: {}", e);
                ApiError::internal("Database error")
            })?
            .unwrap_or(thread);

        // Get author info
        let author = user_repo
            .get_by_id(claims.sub)
            .ok()
            .flatten()
            .map(|u| AuthorInfo {
                id: u.id,
                username: u.username,
                nickname: u.nickname,
            })
            .unwrap_or_else(|| AuthorInfo {
                id: claims.sub,
                username: claims.username.clone(),
                nickname: claims.username.clone(),
            });

        (thread, author)
    };

    let response = ThreadResponse {
        id: thread.id,
        board_id: thread.board_id,
        title: thread.title,
        author,
        post_count: thread.post_count,
        created_at: thread.created_at,
        updated_at: thread.updated_at,
    };

    Ok(Json(ApiResponse::new(response)))
}

/// GET /api/boards/:id/posts - List posts in a flat board.
pub async fn list_flat_posts(
    State(state): State<Arc<AppState>>,
    OptionalAuthUser(auth): OptionalAuthUser,
    Path(board_id): Path<i64>,
    Query(pagination): Query<PaginationQuery>,
) -> Result<Json<PaginatedResponse<PostResponse>>, ApiError> {
    let user_role = auth
        .map(|c| Role::from_str(&c.role).unwrap_or(Role::Guest))
        .unwrap_or(Role::Guest);

    let (offset, limit) = pagination.to_offset_limit();

    let (posts, total) = {
        let db = state.db.lock().await;
        let board_repo = BoardRepository::new(&*db);
        let post_repo = PostRepository::new(&*db);

        let board = board_repo
            .get_by_id(board_id)
            .map_err(|e| {
                tracing::error!("Failed to get board: {}", e);
                ApiError::internal("Database error")
            })?
            .ok_or_else(|| ApiError::not_found("Board not found"))?;

        if !board.can_read(user_role) {
            return Err(ApiError::forbidden("Access denied"));
        }

        if board.board_type != BoardType::Flat {
            return Err(ApiError::bad_request(
                "This board does not support flat posts",
            ));
        }

        let posts = post_repo
            .list_by_flat_board_paginated(board_id, offset, limit)
            .map_err(|e| {
                tracing::error!("Failed to list posts: {}", e);
                ApiError::internal("Database error")
            })?;

        let total = post_repo.count_by_flat_board(board_id).map_err(|e| {
            tracing::error!("Failed to count posts: {}", e);
            ApiError::internal("Database error")
        })?;

        (posts, total)
    };

    // Get author info for each post
    let responses = {
        let db = state.db.lock().await;
        let user_repo = UserRepository::new(&*db);

        posts
            .into_iter()
            .map(|p| {
                let author = user_repo
                    .get_by_id(p.author_id)
                    .ok()
                    .flatten()
                    .map(|u| AuthorInfo {
                        id: u.id,
                        username: u.username,
                        nickname: u.nickname,
                    })
                    .unwrap_or_else(|| AuthorInfo {
                        id: p.author_id,
                        username: "unknown".to_string(),
                        nickname: "Unknown".to_string(),
                    });

                PostResponse {
                    id: p.id,
                    board_id: p.board_id,
                    thread_id: p.thread_id,
                    author,
                    title: p.title,
                    body: p.body,
                    created_at: p.created_at,
                }
            })
            .collect()
    };

    Ok(Json(PaginatedResponse::new(
        responses,
        pagination.page,
        pagination.per_page,
        total as u64,
    )))
}

/// POST /api/boards/:id/posts - Create a post in a flat board.
pub async fn create_flat_post(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser,
    Path(board_id): Path<i64>,
    Json(req): Json<CreateFlatPostRequest>,
) -> Result<Json<ApiResponse<PostResponse>>, ApiError> {
    let user_role = Role::from_str(&claims.role).unwrap_or(Role::Guest);

    // Validate input
    if req.title.trim().is_empty() {
        return Err(ApiError::bad_request("Title is required"));
    }
    if req.body.trim().is_empty() {
        return Err(ApiError::bad_request("Body is required"));
    }

    let (post, author) = {
        let db = state.db.lock().await;
        let board_repo = BoardRepository::new(&*db);
        let post_repo = PostRepository::new(&*db);
        let user_repo = UserRepository::new(&*db);

        // Check board access
        let board = board_repo
            .get_by_id(board_id)
            .map_err(|e| {
                tracing::error!("Failed to get board: {}", e);
                ApiError::internal("Database error")
            })?
            .ok_or_else(|| ApiError::not_found("Board not found"))?;

        if !board.can_write(user_role) {
            return Err(ApiError::forbidden("Write access denied"));
        }

        if board.board_type != BoardType::Flat {
            return Err(ApiError::bad_request(
                "This board does not support flat posts",
            ));
        }

        // Create post
        let new_post = NewFlatPost::new(board_id, claims.sub, &req.title, &req.body);
        let post = post_repo.create_flat_post(&new_post).map_err(|e| {
            tracing::error!("Failed to create post: {}", e);
            ApiError::internal("Failed to create post")
        })?;

        // Get author info
        let author = user_repo
            .get_by_id(claims.sub)
            .ok()
            .flatten()
            .map(|u| AuthorInfo {
                id: u.id,
                username: u.username,
                nickname: u.nickname,
            })
            .unwrap_or_else(|| AuthorInfo {
                id: claims.sub,
                username: claims.username.clone(),
                nickname: claims.username.clone(),
            });

        (post, author)
    };

    let response = PostResponse {
        id: post.id,
        board_id: post.board_id,
        thread_id: post.thread_id,
        author,
        title: post.title,
        body: post.body,
        created_at: post.created_at,
    };

    Ok(Json(ApiResponse::new(response)))
}

/// GET /api/threads/:id - Get thread details.
pub async fn get_thread(
    State(state): State<Arc<AppState>>,
    OptionalAuthUser(auth): OptionalAuthUser,
    Path(thread_id): Path<i64>,
) -> Result<Json<ApiResponse<ThreadResponse>>, ApiError> {
    let user_role = auth
        .map(|c| Role::from_str(&c.role).unwrap_or(Role::Guest))
        .unwrap_or(Role::Guest);

    let (thread, author) = {
        let db = state.db.lock().await;
        let thread_repo = ThreadRepository::new(&*db);
        let board_repo = BoardRepository::new(&*db);
        let user_repo = UserRepository::new(&*db);

        let thread = thread_repo
            .get_by_id(thread_id)
            .map_err(|e| {
                tracing::error!("Failed to get thread: {}", e);
                ApiError::internal("Database error")
            })?
            .ok_or_else(|| ApiError::not_found("Thread not found"))?;

        // Check board access
        let board = board_repo
            .get_by_id(thread.board_id)
            .map_err(|e| {
                tracing::error!("Failed to get board: {}", e);
                ApiError::internal("Database error")
            })?
            .ok_or_else(|| ApiError::not_found("Board not found"))?;

        if !board.can_read(user_role) {
            return Err(ApiError::forbidden("Access denied"));
        }

        let author = user_repo
            .get_by_id(thread.author_id)
            .ok()
            .flatten()
            .map(|u| AuthorInfo {
                id: u.id,
                username: u.username,
                nickname: u.nickname,
            })
            .unwrap_or_else(|| AuthorInfo {
                id: thread.author_id,
                username: "unknown".to_string(),
                nickname: "Unknown".to_string(),
            });

        (thread, author)
    };

    let response = ThreadResponse {
        id: thread.id,
        board_id: thread.board_id,
        title: thread.title,
        author,
        post_count: thread.post_count,
        created_at: thread.created_at,
        updated_at: thread.updated_at,
    };

    Ok(Json(ApiResponse::new(response)))
}

/// GET /api/threads/:id/posts - List posts in a thread.
pub async fn list_thread_posts(
    State(state): State<Arc<AppState>>,
    OptionalAuthUser(auth): OptionalAuthUser,
    Path(thread_id): Path<i64>,
    Query(pagination): Query<PaginationQuery>,
) -> Result<Json<PaginatedResponse<PostResponse>>, ApiError> {
    let user_role = auth
        .map(|c| Role::from_str(&c.role).unwrap_or(Role::Guest))
        .unwrap_or(Role::Guest);

    let (offset, limit) = pagination.to_offset_limit();

    let (posts, total) = {
        let db = state.db.lock().await;
        let thread_repo = ThreadRepository::new(&*db);
        let board_repo = BoardRepository::new(&*db);
        let post_repo = PostRepository::new(&*db);

        let thread = thread_repo
            .get_by_id(thread_id)
            .map_err(|e| {
                tracing::error!("Failed to get thread: {}", e);
                ApiError::internal("Database error")
            })?
            .ok_or_else(|| ApiError::not_found("Thread not found"))?;

        // Check board access
        let board = board_repo
            .get_by_id(thread.board_id)
            .map_err(|e| {
                tracing::error!("Failed to get board: {}", e);
                ApiError::internal("Database error")
            })?
            .ok_or_else(|| ApiError::not_found("Board not found"))?;

        if !board.can_read(user_role) {
            return Err(ApiError::forbidden("Access denied"));
        }

        let posts = post_repo
            .list_by_thread_paginated(thread_id, offset, limit)
            .map_err(|e| {
                tracing::error!("Failed to list posts: {}", e);
                ApiError::internal("Database error")
            })?;

        let total = post_repo.count_by_thread(thread_id).map_err(|e| {
            tracing::error!("Failed to count posts: {}", e);
            ApiError::internal("Database error")
        })?;

        (posts, total)
    };

    // Get author info for each post
    let responses = {
        let db = state.db.lock().await;
        let user_repo = UserRepository::new(&*db);

        posts
            .into_iter()
            .map(|p| {
                let author = user_repo
                    .get_by_id(p.author_id)
                    .ok()
                    .flatten()
                    .map(|u| AuthorInfo {
                        id: u.id,
                        username: u.username,
                        nickname: u.nickname,
                    })
                    .unwrap_or_else(|| AuthorInfo {
                        id: p.author_id,
                        username: "unknown".to_string(),
                        nickname: "Unknown".to_string(),
                    });

                PostResponse {
                    id: p.id,
                    board_id: p.board_id,
                    thread_id: p.thread_id,
                    author,
                    title: p.title,
                    body: p.body,
                    created_at: p.created_at,
                }
            })
            .collect()
    };

    Ok(Json(PaginatedResponse::new(
        responses,
        pagination.page,
        pagination.per_page,
        total as u64,
    )))
}

/// POST /api/threads/:id/posts - Reply to a thread.
pub async fn create_thread_post(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser,
    Path(thread_id): Path<i64>,
    Json(req): Json<CreatePostRequest>,
) -> Result<Json<ApiResponse<PostResponse>>, ApiError> {
    let user_role = Role::from_str(&claims.role).unwrap_or(Role::Guest);

    // Validate input
    if req.body.trim().is_empty() {
        return Err(ApiError::bad_request("Body is required"));
    }

    let (post, author) = {
        let db = state.db.lock().await;
        let thread_repo = ThreadRepository::new(&*db);
        let board_repo = BoardRepository::new(&*db);
        let post_repo = PostRepository::new(&*db);
        let user_repo = UserRepository::new(&*db);

        let thread = thread_repo
            .get_by_id(thread_id)
            .map_err(|e| {
                tracing::error!("Failed to get thread: {}", e);
                ApiError::internal("Database error")
            })?
            .ok_or_else(|| ApiError::not_found("Thread not found"))?;

        // Check board access
        let board = board_repo
            .get_by_id(thread.board_id)
            .map_err(|e| {
                tracing::error!("Failed to get board: {}", e);
                ApiError::internal("Database error")
            })?
            .ok_or_else(|| ApiError::not_found("Board not found"))?;

        if !board.can_write(user_role) {
            return Err(ApiError::forbidden("Write access denied"));
        }

        // Create post
        let new_post = NewThreadPost::new(thread.board_id, thread_id, claims.sub, &req.body);
        let post = post_repo.create_thread_post(&new_post).map_err(|e| {
            tracing::error!("Failed to create post: {}", e);
            ApiError::internal("Failed to create post")
        })?;

        // Update thread
        thread_repo.touch_and_increment(thread_id).map_err(|e| {
            tracing::error!("Failed to update thread: {}", e);
            ApiError::internal("Database error")
        })?;

        // Get author info
        let author = user_repo
            .get_by_id(claims.sub)
            .ok()
            .flatten()
            .map(|u| AuthorInfo {
                id: u.id,
                username: u.username,
                nickname: u.nickname,
            })
            .unwrap_or_else(|| AuthorInfo {
                id: claims.sub,
                username: claims.username.clone(),
                nickname: claims.username.clone(),
            });

        (post, author)
    };

    let response = PostResponse {
        id: post.id,
        board_id: post.board_id,
        thread_id: post.thread_id,
        author,
        title: post.title,
        body: post.body,
        created_at: post.created_at,
    };

    Ok(Json(ApiResponse::new(response)))
}

/// DELETE /api/posts/:id - Delete a post.
pub async fn delete_post(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser,
    Path(post_id): Path<i64>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    let user_role = Role::from_str(&claims.role).unwrap_or(Role::Guest);

    {
        let db = state.db.lock().await;
        let post_repo = PostRepository::new(&*db);
        let thread_repo = ThreadRepository::new(&*db);

        let post = post_repo
            .get_by_id(post_id)
            .map_err(|e| {
                tracing::error!("Failed to get post: {}", e);
                ApiError::internal("Database error")
            })?
            .ok_or_else(|| ApiError::not_found("Post not found"))?;

        // Only author or admin can delete
        if post.author_id != claims.sub && user_role < Role::SubOp {
            return Err(ApiError::forbidden("You can only delete your own posts"));
        }

        // Delete the post
        post_repo.delete(post_id).map_err(|e| {
            tracing::error!("Failed to delete post: {}", e);
            ApiError::internal("Failed to delete post")
        })?;

        // Update thread post count if it's a thread post
        if let Some(thread_id) = post.thread_id {
            thread_repo.decrement_post_count(thread_id).map_err(|e| {
                tracing::error!("Failed to update thread: {}", e);
                ApiError::internal("Database error")
            })?;
        }
    }

    Ok(Json(ApiResponse::new(())))
}

// Helper to parse Role from string
trait RoleExt {
    fn from_str(s: &str) -> Option<Role>;
}

impl RoleExt for Role {
    fn from_str(s: &str) -> Option<Role> {
        match s.to_lowercase().as_str() {
            "guest" => Some(Role::Guest),
            "member" => Some(Role::Member),
            "subop" => Some(Role::SubOp),
            "sysop" => Some(Role::SysOp),
            _ => None,
        }
    }
}
