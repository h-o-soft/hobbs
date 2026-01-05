//! User handlers for Web API.

use axum::{
    extract::{Path, Query, State},
    Json,
};
use std::sync::Arc;

use crate::auth::{hash_password, verify_password};
use crate::db::{UserRepository, UserUpdate};
use crate::web::dto::{
    ApiResponse, ChangePasswordRequest, PaginatedResponse, PaginationQuery, UpdateProfileRequest,
    UserDetailResponse, UserListResponse,
};
use crate::web::error::ApiError;
use crate::web::handlers::AppState;
use crate::web::middleware::AuthUser;

/// GET /api/users - List all users (paginated).
pub async fn list_users(
    State(state): State<Arc<AppState>>,
    AuthUser(_claims): AuthUser,
    Query(pagination): Query<PaginationQuery>,
) -> Result<Json<PaginatedResponse<UserListResponse>>, ApiError> {
    let (offset, limit) = pagination.to_offset_limit();

    let (users, total) = {
        let db = state.db.lock().await;
        let user_repo = UserRepository::new(&*db);

        let all_users = user_repo.list_active().map_err(|e| {
            tracing::error!("Failed to list users: {}", e);
            ApiError::internal("Failed to list users")
        })?;

        let total = all_users.len() as i64;

        // Manual pagination
        let users: Vec<_> = all_users
            .into_iter()
            .skip(offset as usize)
            .take(limit as usize)
            .collect();

        (users, total)
    };

    let responses: Vec<_> = users
        .into_iter()
        .map(|u| UserListResponse {
            id: u.id,
            username: u.username,
            nickname: u.nickname,
            role: u.role.as_str().to_string(),
            last_login_at: u.last_login,
        })
        .collect();

    Ok(Json(PaginatedResponse::new(
        responses,
        pagination.page,
        pagination.per_page,
        total as u64,
    )))
}

/// GET /api/users/:id - Get user profile by ID.
pub async fn get_user(
    State(state): State<Arc<AppState>>,
    AuthUser(_claims): AuthUser,
    Path(user_id): Path<i64>,
) -> Result<Json<ApiResponse<UserDetailResponse>>, ApiError> {
    let user = {
        let db = state.db.lock().await;
        let user_repo = UserRepository::new(&*db);

        user_repo
            .get_by_id(user_id)
            .map_err(|e| {
                tracing::error!("Failed to get user: {}", e);
                ApiError::internal("Failed to get user")
            })?
            .ok_or_else(|| ApiError::not_found("User not found"))?
    };

    // Only show active users
    if !user.is_active {
        return Err(ApiError::not_found("User not found"));
    }

    let response = UserDetailResponse {
        id: user.id,
        username: user.username,
        nickname: user.nickname,
        role: user.role.as_str().to_string(),
        profile: user.profile,
        created_at: user.created_at,
        last_login_at: user.last_login,
    };

    Ok(Json(ApiResponse::new(response)))
}

/// GET /api/users/me - Get current user's profile.
pub async fn get_my_profile(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser,
) -> Result<Json<ApiResponse<UserDetailResponse>>, ApiError> {
    let user = {
        let db = state.db.lock().await;
        let user_repo = UserRepository::new(&*db);

        user_repo
            .get_by_id(claims.sub)
            .map_err(|e| {
                tracing::error!("Failed to get user: {}", e);
                ApiError::internal("Failed to get user")
            })?
            .ok_or_else(|| ApiError::not_found("User not found"))?
    };

    let response = UserDetailResponse {
        id: user.id,
        username: user.username,
        nickname: user.nickname,
        role: user.role.as_str().to_string(),
        profile: user.profile,
        created_at: user.created_at,
        last_login_at: user.last_login,
    };

    Ok(Json(ApiResponse::new(response)))
}

/// PUT /api/users/me - Update current user's profile.
pub async fn update_my_profile(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser,
    Json(req): Json<UpdateProfileRequest>,
) -> Result<Json<ApiResponse<UserDetailResponse>>, ApiError> {
    // Build update struct
    let mut update = UserUpdate::new();

    if let Some(nickname) = req.nickname {
        if nickname.trim().is_empty() {
            return Err(ApiError::bad_request("Nickname cannot be empty"));
        }
        if nickname.len() > 20 {
            return Err(ApiError::bad_request(
                "Nickname must be 20 characters or less",
            ));
        }
        update = update.nickname(nickname);
    }

    if let Some(email) = req.email {
        let email_opt = if email.trim().is_empty() {
            None
        } else {
            Some(email)
        };
        update = update.email(email_opt);
    }

    if let Some(profile) = req.profile {
        let profile_opt = if profile.trim().is_empty() {
            None
        } else {
            Some(profile)
        };
        update = update.profile(profile_opt);
    }

    let user = {
        let db = state.db.lock().await;
        let user_repo = UserRepository::new(&*db);

        user_repo
            .update(claims.sub, &update)
            .map_err(|e| {
                tracing::error!("Failed to update profile: {}", e);
                ApiError::internal("Failed to update profile")
            })?
            .ok_or_else(|| ApiError::not_found("User not found"))?
    };

    let response = UserDetailResponse {
        id: user.id,
        username: user.username,
        nickname: user.nickname,
        role: user.role.as_str().to_string(),
        profile: user.profile,
        created_at: user.created_at,
        last_login_at: user.last_login,
    };

    Ok(Json(ApiResponse::new(response)))
}

/// POST /api/users/me/password - Change current user's password.
pub async fn change_password(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser,
    Json(req): Json<ChangePasswordRequest>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    // Validate new password
    if req.new_password.len() < 8 {
        return Err(ApiError::bad_request(
            "Password must be at least 8 characters",
        ));
    }
    if req.new_password.len() > 128 {
        return Err(ApiError::bad_request(
            "Password must be 128 characters or less",
        ));
    }

    {
        let db = state.db.lock().await;
        let user_repo = UserRepository::new(&*db);

        // Get current user
        let user = user_repo
            .get_by_id(claims.sub)
            .map_err(|e| {
                tracing::error!("Failed to get user: {}", e);
                ApiError::internal("Failed to get user")
            })?
            .ok_or_else(|| ApiError::not_found("User not found"))?;

        // Verify current password
        verify_password(&req.current_password, &user.password).map_err(|_| {
            ApiError::unauthorized("Current password is incorrect")
        })?;

        // Hash new password
        let new_hash = hash_password(&req.new_password).map_err(|e| {
            tracing::error!("Failed to hash password: {}", e);
            ApiError::internal("Failed to update password")
        })?;

        // Update password
        let update = UserUpdate::new().password(new_hash);
        user_repo.update(claims.sub, &update).map_err(|e| {
            tracing::error!("Failed to update password: {}", e);
            ApiError::internal("Failed to update password")
        })?;
    }

    Ok(Json(ApiResponse::new(())))
}
