//! Admin handlers for Web API.

use axum::{
    extract::{Path, Query, State},
    Json,
};
use std::sync::Arc;
use utoipa;

use crate::auth::hash_password;
use crate::board::{BoardRepository, BoardType, BoardUpdate, NewBoard};
use crate::datetime::to_rfc3339;
use crate::db::{Role, UserRepository, UserUpdate};
use crate::file::{FileRepository, FolderRepository, FolderUpdate, NewFolder};
use crate::web::dto::{
    AdminBoardResponse, AdminCreateBoardRequest, AdminCreateFolderRequest, AdminFolderResponse,
    AdminResetPasswordRequest, AdminUpdateBoardRequest, AdminUpdateFolderRequest,
    AdminUpdateRoleRequest, AdminUpdateStatusRequest, AdminUpdateUserRequest, AdminUserResponse,
    ApiResponse, PaginatedResponse, PaginationQuery,
};
use crate::web::error::ApiError;
use crate::web::handlers::AppState;
use crate::web::middleware::AuthUser;

/// Helper to check SubOp or higher permission
fn require_subop(claims: &crate::web::middleware::JwtClaims) -> Result<Role, ApiError> {
    let role: Role = claims.role.parse().unwrap_or(Role::Guest);
    if role < Role::SubOp {
        return Err(ApiError::forbidden("Admin access required"));
    }
    Ok(role)
}

/// Helper to check SysOp permission
fn require_sysop(claims: &crate::web::middleware::JwtClaims) -> Result<Role, ApiError> {
    let role: Role = claims.role.parse().unwrap_or(Role::Guest);
    if role < Role::SysOp {
        return Err(ApiError::forbidden("SysOp access required"));
    }
    Ok(role)
}

// ============================================================================
// User Management
// ============================================================================

/// GET /api/admin/users - List all users (admin).
#[utoipa::path(
    get,
    path = "/admin/users",
    tag = "admin",
    params(
        ("page" = Option<u32>, Query, description = "Page number"),
        ("per_page" = Option<u32>, Query, description = "Items per page")
    ),
    responses(
        (status = 200, description = "List of all users", body = Vec<AdminUserResponse>),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Admin access required")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn admin_list_users(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser,
    Query(pagination): Query<PaginationQuery>,
) -> Result<Json<PaginatedResponse<AdminUserResponse>>, ApiError> {
    require_subop(&claims)?;
    let (offset, limit) = pagination.to_offset_limit();

    let user_repo = UserRepository::new(state.db.pool());

    let all_users = user_repo.list_all().await.map_err(|e| {
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

    let responses: Vec<_> = users
        .into_iter()
        .map(|u| AdminUserResponse {
            id: u.id,
            username: u.username,
            nickname: u.nickname,
            role: u.role.as_str().to_string(),
            email: u.email,
            is_active: u.is_active,
            created_at: to_rfc3339(&u.created_at),
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

/// PUT /api/admin/users/:id - Update user info (admin).
#[utoipa::path(
    put,
    path = "/admin/users/{id}",
    tag = "admin",
    params(
        ("id" = i64, Path, description = "User ID")
    ),
    request_body = AdminUpdateUserRequest,
    responses(
        (status = 200, description = "User updated", body = AdminUserResponse),
        (status = 400, description = "Invalid input"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Admin access required"),
        (status = 404, description = "User not found")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn admin_update_user(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser,
    Path(user_id): Path<i64>,
    Json(req): Json<AdminUpdateUserRequest>,
) -> Result<Json<ApiResponse<AdminUserResponse>>, ApiError> {
    require_subop(&claims)?;

    // Build update
    let mut update = UserUpdate::new();

    if let Some(nickname) = req.nickname {
        if nickname.trim().is_empty() {
            return Err(ApiError::bad_request("Nickname cannot be empty"));
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

    let user_repo = UserRepository::new(state.db.pool());
    let user = user_repo
        .update(user_id, &update)
        .await
        .map_err(|e| {
            tracing::error!("Failed to update user: {}", e);
            ApiError::internal("Failed to update user")
        })?
        .ok_or_else(|| ApiError::not_found("User not found"))?;

    let response = AdminUserResponse {
        id: user.id,
        username: user.username,
        nickname: user.nickname,
        role: user.role.as_str().to_string(),
        email: user.email,
        is_active: user.is_active,
        created_at: to_rfc3339(&user.created_at),
        last_login_at: user.last_login,
    };

    Ok(Json(ApiResponse::new(response)))
}

/// PUT /api/admin/users/:id/role - Update user role (SysOp only).
#[utoipa::path(
    put,
    path = "/admin/users/{id}/role",
    tag = "admin",
    params(
        ("id" = i64, Path, description = "User ID")
    ),
    request_body = AdminUpdateRoleRequest,
    responses(
        (status = 200, description = "Role updated", body = AdminUserResponse),
        (status = 400, description = "Invalid role or cannot change own role"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "SysOp access required"),
        (status = 404, description = "User not found")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn admin_update_role(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser,
    Path(user_id): Path<i64>,
    Json(req): Json<AdminUpdateRoleRequest>,
) -> Result<Json<ApiResponse<AdminUserResponse>>, ApiError> {
    require_sysop(&claims)?;

    // Cannot change own role
    if user_id == claims.sub {
        return Err(ApiError::bad_request("Cannot change your own role"));
    }

    let new_role: Role = req
        .role
        .parse()
        .map_err(|_| ApiError::bad_request("Invalid role"))?;

    let update = UserUpdate::new().role(new_role);

    let user_repo = UserRepository::new(state.db.pool());
    let user = user_repo
        .update(user_id, &update)
        .await
        .map_err(|e| {
            tracing::error!("Failed to update user role: {}", e);
            ApiError::internal("Failed to update user role")
        })?
        .ok_or_else(|| ApiError::not_found("User not found"))?;

    let response = AdminUserResponse {
        id: user.id,
        username: user.username,
        nickname: user.nickname,
        role: user.role.as_str().to_string(),
        email: user.email,
        is_active: user.is_active,
        created_at: to_rfc3339(&user.created_at),
        last_login_at: user.last_login,
    };

    Ok(Json(ApiResponse::new(response)))
}

/// PUT /api/admin/users/:id/status - Update user status (admin).
#[utoipa::path(
    put,
    path = "/admin/users/{id}/status",
    tag = "admin",
    params(
        ("id" = i64, Path, description = "User ID")
    ),
    request_body = AdminUpdateStatusRequest,
    responses(
        (status = 200, description = "Status updated", body = AdminUserResponse),
        (status = 400, description = "Cannot change own status"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Admin access required"),
        (status = 404, description = "User not found")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn admin_update_status(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser,
    Path(user_id): Path<i64>,
    Json(req): Json<AdminUpdateStatusRequest>,
) -> Result<Json<ApiResponse<AdminUserResponse>>, ApiError> {
    require_subop(&claims)?;

    // Cannot change own status
    if user_id == claims.sub {
        return Err(ApiError::bad_request("Cannot change your own status"));
    }

    let update = UserUpdate::new().is_active(req.is_active);

    let user_repo = UserRepository::new(state.db.pool());
    let user = user_repo
        .update(user_id, &update)
        .await
        .map_err(|e| {
            tracing::error!("Failed to update user status: {}", e);
            ApiError::internal("Failed to update user status")
        })?
        .ok_or_else(|| ApiError::not_found("User not found"))?;

    let response = AdminUserResponse {
        id: user.id,
        username: user.username,
        nickname: user.nickname,
        role: user.role.as_str().to_string(),
        email: user.email,
        is_active: user.is_active,
        created_at: to_rfc3339(&user.created_at),
        last_login_at: user.last_login,
    };

    Ok(Json(ApiResponse::new(response)))
}

/// POST /api/admin/users/:id/reset-password - Reset user password (admin).
#[utoipa::path(
    post,
    path = "/admin/users/{id}/reset-password",
    tag = "admin",
    params(
        ("id" = i64, Path, description = "User ID")
    ),
    request_body = AdminResetPasswordRequest,
    responses(
        (status = 200, description = "Password reset"),
        (status = 400, description = "Password too short"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Admin access required"),
        (status = 404, description = "User not found")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn admin_reset_password(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser,
    Path(user_id): Path<i64>,
    Json(req): Json<AdminResetPasswordRequest>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    require_subop(&claims)?;

    // Validate password
    if req.new_password.len() < 8 {
        return Err(ApiError::bad_request(
            "Password must be at least 8 characters",
        ));
    }

    let password_hash = hash_password(&req.new_password).map_err(|e| {
        tracing::error!("Failed to hash password: {}", e);
        ApiError::internal("Failed to hash password")
    })?;

    let update = UserUpdate::new().password(password_hash);

    let user_repo = UserRepository::new(state.db.pool());
    user_repo
        .update(user_id, &update)
        .await
        .map_err(|e| {
            tracing::error!("Failed to reset password: {}", e);
            ApiError::internal("Failed to reset password")
        })?
        .ok_or_else(|| ApiError::not_found("User not found"))?;

    Ok(Json(ApiResponse::new(())))
}

// ============================================================================
// Board Management
// ============================================================================

/// GET /api/admin/boards - List all boards (admin).
#[utoipa::path(
    get,
    path = "/admin/boards",
    tag = "admin",
    responses(
        (status = 200, description = "List of all boards", body = Vec<AdminBoardResponse>),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Admin access required")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn admin_list_boards(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser,
) -> Result<Json<ApiResponse<Vec<AdminBoardResponse>>>, ApiError> {
    require_subop(&claims)?;

    let board_repo = BoardRepository::new(state.db.pool());
    let boards = board_repo.list_all().await.map_err(|e| {
        tracing::error!("Failed to list boards: {}", e);
        ApiError::internal("Failed to list boards")
    })?;

    let responses: Vec<_> = boards
        .into_iter()
        .map(|b| AdminBoardResponse {
            id: b.id,
            name: b.name,
            description: b.description,
            board_type: b.board_type.as_str().to_string(),
            min_read_role: b.min_read_role.as_str().to_string(),
            min_write_role: b.min_write_role.as_str().to_string(),
            sort_order: b.sort_order,
            is_active: b.is_active,
            created_at: to_rfc3339(&b.created_at),
        })
        .collect();

    Ok(Json(ApiResponse::new(responses)))
}

/// POST /api/admin/boards - Create a board (admin).
#[utoipa::path(
    post,
    path = "/admin/boards",
    tag = "admin",
    request_body = AdminCreateBoardRequest,
    responses(
        (status = 200, description = "Board created", body = AdminBoardResponse),
        (status = 400, description = "Invalid input"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Admin access required")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn admin_create_board(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser,
    Json(req): Json<AdminCreateBoardRequest>,
) -> Result<Json<ApiResponse<AdminBoardResponse>>, ApiError> {
    require_subop(&claims)?;

    // Validate name
    if req.name.trim().is_empty() {
        return Err(ApiError::bad_request("Board name is required"));
    }

    let board_type: BoardType = req
        .board_type
        .parse()
        .map_err(|_| ApiError::bad_request("Invalid board type"))?;

    let min_read_role: Role = req
        .min_read_role
        .parse()
        .map_err(|_| ApiError::bad_request("Invalid min_read_role"))?;

    let min_write_role: Role = req
        .min_write_role
        .parse()
        .map_err(|_| ApiError::bad_request("Invalid min_write_role"))?;

    let mut new_board = NewBoard::new(&req.name)
        .with_board_type(board_type)
        .with_min_read_role(min_read_role)
        .with_min_write_role(min_write_role)
        .with_sort_order(req.sort_order);

    if let Some(ref desc) = req.description {
        new_board = new_board.with_description(desc);
    }

    let board_repo = BoardRepository::new(state.db.pool());
    let board = board_repo.create(&new_board).await.map_err(|e| {
        tracing::error!("Failed to create board: {}", e);
        ApiError::internal("Failed to create board")
    })?;

    let response = AdminBoardResponse {
        id: board.id,
        name: board.name,
        description: board.description,
        board_type: board.board_type.as_str().to_string(),
        min_read_role: board.min_read_role.as_str().to_string(),
        min_write_role: board.min_write_role.as_str().to_string(),
        sort_order: board.sort_order,
        is_active: board.is_active,
        created_at: to_rfc3339(&board.created_at),
    };

    Ok(Json(ApiResponse::new(response)))
}

/// PUT /api/admin/boards/:id - Update a board (admin).
#[utoipa::path(
    put,
    path = "/admin/boards/{id}",
    tag = "admin",
    params(
        ("id" = i64, Path, description = "Board ID")
    ),
    request_body = AdminUpdateBoardRequest,
    responses(
        (status = 200, description = "Board updated", body = AdminBoardResponse),
        (status = 400, description = "Invalid input"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Admin access required"),
        (status = 404, description = "Board not found")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn admin_update_board(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser,
    Path(board_id): Path<i64>,
    Json(req): Json<AdminUpdateBoardRequest>,
) -> Result<Json<ApiResponse<AdminBoardResponse>>, ApiError> {
    require_subop(&claims)?;

    let mut update = BoardUpdate::new();

    if let Some(ref name) = req.name {
        if name.trim().is_empty() {
            return Err(ApiError::bad_request("Board name cannot be empty"));
        }
        update = update.name(name);
    }

    if let Some(ref description) = req.description {
        update = update.description(description.clone());
    }

    if let Some(ref board_type) = req.board_type {
        let bt: BoardType = board_type
            .parse()
            .map_err(|_| ApiError::bad_request("Invalid board type"))?;
        update = update.board_type(bt);
    }

    if let Some(ref min_read_role) = req.min_read_role {
        let role: Role = min_read_role
            .parse()
            .map_err(|_| ApiError::bad_request("Invalid min_read_role"))?;
        update = update.min_read_role(role);
    }

    if let Some(ref min_write_role) = req.min_write_role {
        let role: Role = min_write_role
            .parse()
            .map_err(|_| ApiError::bad_request("Invalid min_write_role"))?;
        update = update.min_write_role(role);
    }

    if let Some(sort_order) = req.sort_order {
        update = update.sort_order(sort_order);
    }

    if let Some(is_active) = req.is_active {
        update = update.is_active(is_active);
    }

    let board_repo = BoardRepository::new(state.db.pool());
    let board = board_repo
        .update(board_id, &update)
        .await
        .map_err(|e| {
            tracing::error!("Failed to update board: {}", e);
            ApiError::internal("Failed to update board")
        })?
        .ok_or_else(|| ApiError::not_found("Board not found"))?;

    let response = AdminBoardResponse {
        id: board.id,
        name: board.name,
        description: board.description,
        board_type: board.board_type.as_str().to_string(),
        min_read_role: board.min_read_role.as_str().to_string(),
        min_write_role: board.min_write_role.as_str().to_string(),
        sort_order: board.sort_order,
        is_active: board.is_active,
        created_at: to_rfc3339(&board.created_at),
    };

    Ok(Json(ApiResponse::new(response)))
}

/// DELETE /api/admin/boards/:id - Delete a board (admin).
#[utoipa::path(
    delete,
    path = "/admin/boards/{id}",
    tag = "admin",
    params(
        ("id" = i64, Path, description = "Board ID")
    ),
    responses(
        (status = 200, description = "Board deleted"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Admin access required"),
        (status = 404, description = "Board not found")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn admin_delete_board(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser,
    Path(board_id): Path<i64>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    require_subop(&claims)?;

    let board_repo = BoardRepository::new(state.db.pool());
    let deleted = board_repo.delete(board_id).await.map_err(|e| {
        tracing::error!("Failed to delete board: {}", e);
        ApiError::internal("Failed to delete board")
    })?;

    if !deleted {
        return Err(ApiError::not_found("Board not found"));
    }

    Ok(Json(ApiResponse::new(())))
}

// ============================================================================
// Folder Management
// ============================================================================

/// GET /api/admin/folders - List all folders (admin).
#[utoipa::path(
    get,
    path = "/admin/folders",
    tag = "admin",
    responses(
        (status = 200, description = "List of all folders", body = Vec<AdminFolderResponse>),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Admin access required")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn admin_list_folders(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser,
) -> Result<Json<ApiResponse<Vec<AdminFolderResponse>>>, ApiError> {
    require_subop(&claims)?;

    let folder_repo = FolderRepository::new(state.db.pool());
    let file_repo = FileRepository::new(state.db.pool());

    // Use SysOp role to get all folders (highest permission)
    let folders = folder_repo.list_accessible(Role::SysOp).await.map_err(|e| {
        tracing::error!("Failed to list folders: {}", e);
        ApiError::internal("Failed to list folders")
    })?;

    let mut responses = Vec::new();
    for f in folders {
        let file_count = file_repo.count_by_folder(f.id).await.unwrap_or(0);

        responses.push(AdminFolderResponse {
            id: f.id,
            name: f.name,
            description: f.description,
            parent_id: f.parent_id,
            permission: f.permission.as_str().to_string(),
            upload_perm: f.upload_perm.as_str().to_string(),
            order_num: f.order_num,
            file_count,
            created_at: to_rfc3339(&f.created_at),
        });
    }

    Ok(Json(ApiResponse::new(responses)))
}

/// POST /api/admin/folders - Create a folder (admin).
#[utoipa::path(
    post,
    path = "/admin/folders",
    tag = "admin",
    request_body = AdminCreateFolderRequest,
    responses(
        (status = 200, description = "Folder created", body = AdminFolderResponse),
        (status = 400, description = "Invalid input"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Admin access required")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn admin_create_folder(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser,
    Json(req): Json<AdminCreateFolderRequest>,
) -> Result<Json<ApiResponse<AdminFolderResponse>>, ApiError> {
    require_subop(&claims)?;

    // Validate name
    if req.name.trim().is_empty() {
        return Err(ApiError::bad_request("Folder name is required"));
    }

    let permission: Role = req
        .permission
        .parse()
        .map_err(|_| ApiError::bad_request("Invalid permission"))?;

    let upload_perm: Role = req
        .upload_perm
        .parse()
        .map_err(|_| ApiError::bad_request("Invalid upload_perm"))?;

    let mut new_folder = NewFolder::new(&req.name)
        .with_permission(permission)
        .with_upload_perm(upload_perm)
        .with_order(req.order_num);

    if let Some(ref desc) = req.description {
        new_folder = new_folder.with_description(desc);
    }

    if let Some(parent_id) = req.parent_id {
        new_folder = new_folder.with_parent(parent_id);
    }

    let folder_repo = FolderRepository::new(state.db.pool());
    let folder = folder_repo.create(&new_folder).await.map_err(|e| {
        tracing::error!("Failed to create folder: {}", e);
        ApiError::internal("Failed to create folder")
    })?;

    let response = AdminFolderResponse {
        id: folder.id,
        name: folder.name,
        description: folder.description,
        parent_id: folder.parent_id,
        permission: folder.permission.as_str().to_string(),
        upload_perm: folder.upload_perm.as_str().to_string(),
        order_num: folder.order_num,
        file_count: 0,
        created_at: to_rfc3339(&folder.created_at),
    };

    Ok(Json(ApiResponse::new(response)))
}

/// PUT /api/admin/folders/:id - Update a folder (admin).
#[utoipa::path(
    put,
    path = "/admin/folders/{id}",
    tag = "admin",
    params(
        ("id" = i64, Path, description = "Folder ID")
    ),
    request_body = AdminUpdateFolderRequest,
    responses(
        (status = 200, description = "Folder updated", body = AdminFolderResponse),
        (status = 400, description = "Invalid input"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Admin access required"),
        (status = 404, description = "Folder not found")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn admin_update_folder(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser,
    Path(folder_id): Path<i64>,
    Json(req): Json<AdminUpdateFolderRequest>,
) -> Result<Json<ApiResponse<AdminFolderResponse>>, ApiError> {
    require_subop(&claims)?;

    let mut update = FolderUpdate::new();

    if let Some(ref name) = req.name {
        if name.trim().is_empty() {
            return Err(ApiError::bad_request("Folder name cannot be empty"));
        }
        update = update.name(name);
    }

    if let Some(ref description) = req.description {
        update = update.description(description.clone());
    }

    if let Some(ref parent_id) = req.parent_id {
        update = update.parent_id(*parent_id);
    }

    if let Some(ref permission) = req.permission {
        let role: Role = permission
            .parse()
            .map_err(|_| ApiError::bad_request("Invalid permission"))?;
        update = update.permission(role);
    }

    if let Some(ref upload_perm) = req.upload_perm {
        let role: Role = upload_perm
            .parse()
            .map_err(|_| ApiError::bad_request("Invalid upload_perm"))?;
        update = update.upload_perm(role);
    }

    if let Some(order_num) = req.order_num {
        update = update.order_num(order_num);
    }

    let folder_repo = FolderRepository::new(state.db.pool());
    let file_repo = FileRepository::new(state.db.pool());

    let folder = folder_repo
        .update(folder_id, &update)
        .await
        .map_err(|e| {
            tracing::error!("Failed to update folder: {}", e);
            ApiError::internal("Failed to update folder")
        })?
        .ok_or_else(|| ApiError::not_found("Folder not found"))?;

    let file_count = file_repo.count_by_folder(folder_id).await.unwrap_or(0);

    let response = AdminFolderResponse {
        id: folder.id,
        name: folder.name,
        description: folder.description,
        parent_id: folder.parent_id,
        permission: folder.permission.as_str().to_string(),
        upload_perm: folder.upload_perm.as_str().to_string(),
        order_num: folder.order_num,
        file_count,
        created_at: to_rfc3339(&folder.created_at),
    };

    Ok(Json(ApiResponse::new(response)))
}

/// DELETE /api/admin/folders/:id - Delete a folder (admin).
#[utoipa::path(
    delete,
    path = "/admin/folders/{id}",
    tag = "admin",
    params(
        ("id" = i64, Path, description = "Folder ID")
    ),
    responses(
        (status = 200, description = "Folder deleted"),
        (status = 400, description = "Cannot delete folder with files"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Admin access required"),
        (status = 404, description = "Folder not found")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn admin_delete_folder(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser,
    Path(folder_id): Path<i64>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    require_subop(&claims)?;

    // Check if folder has files
    let folder_repo = FolderRepository::new(state.db.pool());
    let file_repo = FileRepository::new(state.db.pool());

    let file_count = file_repo.count_by_folder(folder_id).await.unwrap_or(0);

    if file_count > 0 {
        return Err(ApiError::bad_request(
            "Cannot delete folder with files. Delete files first.",
        ));
    }

    let deleted = folder_repo.delete(folder_id).await.map_err(|e| {
        tracing::error!("Failed to delete folder: {}", e);
        ApiError::internal("Failed to delete folder")
    })?;

    if !deleted {
        return Err(ApiError::not_found("Folder not found"));
    }

    Ok(Json(ApiResponse::new(())))
}

// Note: Admin RSS management has been removed.
// RSS is now a personal feature where each user manages their own feeds.
