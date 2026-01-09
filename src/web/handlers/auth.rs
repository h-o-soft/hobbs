//! Authentication handlers.

use axum::{extract::State, Json};
use jsonwebtoken::{encode, EncodingKey, Header};
use std::sync::Arc;
use utoipa;

use crate::chat::ChatRoomManager;
use crate::db::{NewRefreshToken, NewUser, RefreshTokenRepository, UserRepository};
use crate::file::FileStorage;
use crate::mail::MailRepository;
use crate::web::dto::{
    ApiResponse, LoginRequest, LoginResponse, LogoutRequest, MeResponse, RefreshRequest,
    RefreshResponse, RegisterRequest, UserInfo,
};
use crate::web::error::ApiError;
use crate::web::middleware::{AuthUser, JwtClaims};
use crate::{Database, Role};

/// Thread-safe database wrapper for Web API.
pub type SharedDatabase = Arc<Database>;

/// Application state shared across handlers.
#[derive(Clone)]
pub struct AppState {
    /// Database connection.
    pub db: SharedDatabase,
    /// JWT encoding key.
    pub encoding_key: EncodingKey,
    /// Access token expiry in seconds.
    pub access_token_expiry: u64,
    /// Refresh token expiry in days.
    pub refresh_token_expiry: u64,
    /// File storage.
    pub file_storage: Option<Arc<FileStorage>>,
    /// Maximum upload size in bytes.
    pub max_upload_size: u64,
    /// Chat room manager.
    pub chat_manager: Option<Arc<ChatRoomManager>>,
    /// BBS name (from config).
    pub bbs_name: String,
    /// BBS description (from config).
    pub bbs_description: String,
    /// SysOp name (from config).
    pub sysop_name: String,
}

impl AppState {
    /// Create a new application state.
    pub fn new(
        db: SharedDatabase,
        jwt_secret: &str,
        access_expiry: u64,
        refresh_expiry: u64,
    ) -> Self {
        Self {
            db,
            encoding_key: EncodingKey::from_secret(jwt_secret.as_bytes()),
            access_token_expiry: access_expiry,
            refresh_token_expiry: refresh_expiry,
            file_storage: None,
            max_upload_size: 10 * 1024 * 1024, // 10MB default
            chat_manager: None,
            bbs_name: "HOBBS".to_string(),
            bbs_description: "A retro BBS system".to_string(),
            sysop_name: "SysOp".to_string(),
        }
    }

    /// Set BBS configuration.
    pub fn with_bbs_config(
        mut self,
        name: impl Into<String>,
        description: impl Into<String>,
        sysop_name: impl Into<String>,
    ) -> Self {
        self.bbs_name = name.into();
        self.bbs_description = description.into();
        self.sysop_name = sysop_name.into();
        self
    }

    /// Set file storage.
    pub fn with_file_storage(mut self, storage: FileStorage, max_upload_size_mb: u64) -> Self {
        self.file_storage = Some(Arc::new(storage));
        self.max_upload_size = max_upload_size_mb * 1024 * 1024;
        self
    }

    /// Set chat room manager.
    pub fn with_chat_manager(mut self, chat_manager: Arc<ChatRoomManager>) -> Self {
        self.chat_manager = Some(chat_manager);
        self
    }

    /// Generate an access token for a user.
    pub fn generate_access_token(
        &self,
        user_id: i64,
        username: &str,
        role: &Role,
    ) -> Result<String, ApiError> {
        let now = chrono::Utc::now().timestamp() as u64;
        let claims = JwtClaims {
            sub: user_id,
            username: username.to_string(),
            role: format!("{:?}", role).to_lowercase(),
            iat: now,
            exp: now + self.access_token_expiry,
            jti: uuid::Uuid::new_v4().to_string(),
        };

        encode(&Header::default(), &claims, &self.encoding_key).map_err(|e| {
            tracing::error!("Failed to encode JWT: {}", e);
            ApiError::internal("Failed to generate token")
        })
    }

    /// Generate a refresh token.
    pub fn generate_refresh_token(&self) -> String {
        uuid::Uuid::new_v4().to_string()
    }
}

/// POST /api/auth/login - User login.
#[utoipa::path(
    post,
    path = "/auth/login",
    tag = "auth",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Login successful", body = LoginResponse),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Invalid credentials")
    )
)]
pub async fn login(
    State(state): State<Arc<AppState>>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<ApiResponse<LoginResponse>>, ApiError> {
    // Validate input
    if req.username.is_empty() || req.password.is_empty() {
        return Err(ApiError::bad_request("Username and password are required"));
    }

    // Get user from database
    let repo = UserRepository::new(state.db.pool());
    let user = repo
        .get_by_username(&req.username)
        .await
        .map_err(|_| ApiError::invalid_credentials())?
        .ok_or_else(|| ApiError::invalid_credentials())?;

    // Verify password
    crate::verify_password(&req.password, &user.password)
        .map_err(|_| ApiError::invalid_credentials())?;

    // Check if user is active
    if !user.is_active {
        return Err(ApiError::account_disabled());
    }

    // Generate tokens
    let access_token = state.generate_access_token(user.id, &user.username, &user.role)?;
    let refresh_token = state.generate_refresh_token();

    // Store refresh token in database
    let token_repo = RefreshTokenRepository::new(state.db.pool());
    let expires_at =
        chrono::Utc::now() + chrono::Duration::days(state.refresh_token_expiry as i64);
    let new_token = NewRefreshToken {
        user_id: user.id,
        token: refresh_token.clone(),
        expires_at: expires_at.format("%Y-%m-%d %H:%M:%S").to_string(),
    };
    token_repo.create(&new_token).await.map_err(|e| {
        tracing::error!("Failed to store refresh token: {}", e);
        ApiError::internal("Failed to create session")
    })?;

    // Update last login time
    let user_repo = UserRepository::new(state.db.pool());
    let _ = user_repo.update_last_login(user.id).await;

    let response = LoginResponse {
        access_token,
        refresh_token,
        expires_in: state.access_token_expiry,
        user: UserInfo {
            id: user.id,
            username: user.username,
            nickname: user.nickname,
            role: format!("{:?}", user.role).to_lowercase(),
        },
    };

    Ok(Json(ApiResponse::new(response)))
}

/// POST /api/auth/logout - User logout.
#[utoipa::path(
    post,
    path = "/auth/logout",
    tag = "auth",
    request_body = LogoutRequest,
    responses(
        (status = 200, description = "Logout successful")
    )
)]
pub async fn logout(
    State(state): State<Arc<AppState>>,
    Json(req): Json<LogoutRequest>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    // Revoke the refresh token
    let repo = RefreshTokenRepository::new(state.db.pool());
    let _ = repo.revoke(&req.refresh_token).await;

    Ok(Json(ApiResponse::new(())))
}

/// POST /api/auth/refresh - Refresh access token.
#[utoipa::path(
    post,
    path = "/auth/refresh",
    tag = "auth",
    request_body = RefreshRequest,
    responses(
        (status = 200, description = "Token refreshed", body = RefreshResponse),
        (status = 401, description = "Invalid or expired refresh token")
    )
)]
pub async fn refresh(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RefreshRequest>,
) -> Result<Json<ApiResponse<RefreshResponse>>, ApiError> {
    // Validate refresh token
    let token_repo = RefreshTokenRepository::new(state.db.pool());
    let token = token_repo
        .get_valid_token(&req.refresh_token)
        .await
        .map_err(|_| ApiError::internal("Database error"))?
        .ok_or_else(|| ApiError::invalid_refresh_token())?;
    let user_id = token.user_id;

    // Get user info
    let user_repo = UserRepository::new(state.db.pool());
    let user = user_repo
        .get_by_id(user_id)
        .await
        .map_err(|_| ApiError::internal("Database error"))?
        .ok_or_else(|| ApiError::user_not_found())?;

    // Check if user is active
    if !user.is_active {
        return Err(ApiError::account_disabled());
    }

    // Revoke old refresh token
    let _ = token_repo.revoke(&req.refresh_token).await;

    // Generate new tokens
    let access_token = state.generate_access_token(user.id, &user.username, &user.role)?;
    let new_refresh_token = state.generate_refresh_token();

    // Store new refresh token
    let expires_at =
        chrono::Utc::now() + chrono::Duration::days(state.refresh_token_expiry as i64);
    let new_token = NewRefreshToken {
        user_id: user.id,
        token: new_refresh_token.clone(),
        expires_at: expires_at.format("%Y-%m-%d %H:%M:%S").to_string(),
    };
    token_repo.create(&new_token).await.map_err(|e| {
        tracing::error!("Failed to store refresh token: {}", e);
        ApiError::internal("Failed to create session")
    })?;

    let response = RefreshResponse {
        access_token,
        refresh_token: new_refresh_token,
        expires_in: state.access_token_expiry,
    };

    Ok(Json(ApiResponse::new(response)))
}

/// POST /api/auth/register - User registration.
#[utoipa::path(
    post,
    path = "/auth/register",
    tag = "auth",
    request_body = RegisterRequest,
    responses(
        (status = 200, description = "Registration successful", body = LoginResponse),
        (status = 400, description = "Invalid input"),
        (status = 422, description = "Validation error")
    )
)]
pub async fn register(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<ApiResponse<LoginResponse>>, ApiError> {
    // Validate input
    if req.username.is_empty() {
        return Err(ApiError::bad_request("Username is required"));
    }
    if req.password.is_empty() {
        return Err(ApiError::bad_request("Password is required"));
    }
    if req.nickname.is_empty() {
        return Err(ApiError::bad_request("Nickname is required"));
    }

    // Validate password
    crate::validate_password(&req.password)
        .map_err(|e| ApiError::unprocessable(format!("Password error: {}", e)))?;

    // Hash password
    let password_hash = crate::hash_password(&req.password)
        .map_err(|_| ApiError::internal("Failed to hash password"))?;

    // Create user
    let user_repo = UserRepository::new(state.db.pool());

    // Check if this is the first user - make them SysOp
    let user_count = user_repo.count().await.map_err(|e| {
        tracing::error!("Failed to count users: {}", e);
        ApiError::internal("Failed to check user count")
    })?;

    let mut new_user = NewUser::new(&req.username, password_hash, &req.nickname);
    if user_count == 0 {
        new_user = new_user.with_role(Role::SysOp);
        tracing::info!("First user registration - assigning SysOp role to {}", req.username);
    }
    if let Some(ref email) = req.email {
        new_user = new_user.with_email(email);
    }
    let user = user_repo.create(&new_user).await.map_err(|e| {
        let error_msg = e.to_string();
        // Check for unique constraint violation (SQLite: UNIQUE, PostgreSQL: duplicate key)
        if error_msg.contains("UNIQUE") || error_msg.contains("duplicate key") {
            ApiError::username_taken()
        } else {
            tracing::error!("User creation failed: {}", e);
            ApiError::internal("Failed to create user")
        }
    })?;

    // Generate tokens
    let access_token = state.generate_access_token(user.id, &user.username, &user.role)?;
    let refresh_token = state.generate_refresh_token();

    // Store refresh token in database
    let token_repo = RefreshTokenRepository::new(state.db.pool());
    let expires_at =
        chrono::Utc::now() + chrono::Duration::days(state.refresh_token_expiry as i64);
    let new_token = NewRefreshToken {
        user_id: user.id,
        token: refresh_token.clone(),
        expires_at: expires_at.format("%Y-%m-%d %H:%M:%S").to_string(),
    };
    token_repo.create(&new_token).await.map_err(|e| {
        tracing::error!("Failed to store refresh token: {}", e);
        ApiError::internal("Failed to create session")
    })?;

    let response = LoginResponse {
        access_token,
        refresh_token,
        expires_in: state.access_token_expiry,
        user: UserInfo {
            id: user.id,
            username: user.username,
            nickname: user.nickname,
            role: format!("{:?}", user.role).to_lowercase(),
        },
    };

    Ok(Json(ApiResponse::new(response)))
}

/// GET /api/auth/me - Get current user info.
#[utoipa::path(
    get,
    path = "/auth/me",
    tag = "auth",
    responses(
        (status = 200, description = "Current user info", body = MeResponse),
        (status = 401, description = "Unauthorized")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn me(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser,
) -> Result<Json<ApiResponse<MeResponse>>, ApiError> {
    // Get user from database
    let user_repo = UserRepository::new(state.db.pool());
    let user = user_repo
        .get_by_id(claims.sub)
        .await
        .map_err(|_| ApiError::internal("Database error"))?
        .ok_or_else(|| ApiError::not_found("User not found"))?;

    // Get unread mail count
    let mail_repo = MailRepository::new(state.db.pool());
    let unread_count = mail_repo.count_unread(claims.sub).await.unwrap_or(0);

    let response = MeResponse {
        id: user.id,
        username: user.username,
        nickname: user.nickname,
        role: format!("{:?}", user.role).to_lowercase(),
        email: user.email,
        unread_mail_count: unread_count as u64,
        created_at: user.created_at.clone(),
        last_login_at: user.last_login.clone(),
    };

    Ok(Json(ApiResponse::new(response)))
}
