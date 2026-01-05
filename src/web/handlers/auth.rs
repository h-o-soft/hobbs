//! Authentication handlers.

use axum::{extract::State, Json};
use jsonwebtoken::{encode, EncodingKey, Header};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::db::{NewRefreshToken, NewUser, RefreshTokenRepository, UserRepository};
use crate::mail::MailRepository;
use crate::web::dto::{
    ApiResponse, LoginRequest, LoginResponse, LogoutRequest, MeResponse, RefreshRequest,
    RefreshResponse, RegisterRequest, UserInfo,
};
use crate::web::error::ApiError;
use crate::web::middleware::{AuthUser, JwtClaims};
use crate::{Database, Role};

/// Thread-safe database wrapper for Web API.
pub type SharedDatabase = Arc<Mutex<Database>>;

/// Application state shared across handlers.
#[derive(Clone)]
pub struct AppState {
    /// Database connection (wrapped in Mutex for thread safety).
    pub db: SharedDatabase,
    /// JWT encoding key.
    pub encoding_key: EncodingKey,
    /// Access token expiry in seconds.
    pub access_token_expiry: u64,
    /// Refresh token expiry in days.
    pub refresh_token_expiry: u64,
}

impl AppState {
    /// Create a new application state.
    pub fn new(db: SharedDatabase, jwt_secret: &str, access_expiry: u64, refresh_expiry: u64) -> Self {
        Self {
            db,
            encoding_key: EncodingKey::from_secret(jwt_secret.as_bytes()),
            access_token_expiry: access_expiry,
            refresh_token_expiry: refresh_expiry,
        }
    }

    /// Generate an access token for a user.
    pub fn generate_access_token(&self, user_id: i64, username: &str, role: &Role) -> Result<String, ApiError> {
        let now = chrono::Utc::now().timestamp() as u64;
        let claims = JwtClaims {
            sub: user_id,
            username: username.to_string(),
            role: format!("{:?}", role).to_lowercase(),
            iat: now,
            exp: now + self.access_token_expiry,
            jti: uuid::Uuid::new_v4().to_string(),
        };

        encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(|e| {
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
pub async fn login(
    State(state): State<Arc<AppState>>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<ApiResponse<LoginResponse>>, ApiError> {
    // Validate input
    if req.username.is_empty() || req.password.is_empty() {
        return Err(ApiError::bad_request("Username and password are required"));
    }

    // Get user from database
    let user = {
        let db = state.db.lock().await;
        let repo = UserRepository::new(&*db);
        repo.get_by_username(&req.username)
            .map_err(|_| ApiError::unauthorized("Invalid username or password"))?
            .ok_or_else(|| ApiError::unauthorized("Invalid username or password"))?
    };

    // Verify password
    crate::verify_password(&req.password, &user.password)
        .map_err(|_| ApiError::unauthorized("Invalid username or password"))?;

    // Check if user is active
    if !user.is_active {
        return Err(ApiError::forbidden("Account is disabled"));
    }

    // Generate tokens
    let access_token = state.generate_access_token(user.id, &user.username, &user.role)?;
    let refresh_token = state.generate_refresh_token();

    // Store refresh token in database
    {
        let db = state.db.lock().await;
        let repo = RefreshTokenRepository::new(db.conn());
        let expires_at = chrono::Utc::now()
            + chrono::Duration::days(state.refresh_token_expiry as i64);
        let new_token = NewRefreshToken {
            user_id: user.id,
            token: refresh_token.clone(),
            expires_at: expires_at.format("%Y-%m-%d %H:%M:%S").to_string(),
        };
        repo.create(&new_token)
            .map_err(|e| {
                tracing::error!("Failed to store refresh token: {}", e);
                ApiError::internal("Failed to create session")
            })?;
    }

    // Update last login time
    {
        let db = state.db.lock().await;
        let repo = UserRepository::new(&*db);
        let _ = repo.update_last_login(user.id);
    }

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
pub async fn logout(
    State(state): State<Arc<AppState>>,
    Json(req): Json<LogoutRequest>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    // Revoke the refresh token
    let db = state.db.lock().await;
    let repo = RefreshTokenRepository::new(db.conn());
    let _ = repo.revoke(&req.refresh_token);

    Ok(Json(ApiResponse::new(())))
}

/// POST /api/auth/refresh - Refresh access token.
pub async fn refresh(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RefreshRequest>,
) -> Result<Json<ApiResponse<RefreshResponse>>, ApiError> {
    // Validate refresh token
    let user_id = {
        let db = state.db.lock().await;
        let repo = RefreshTokenRepository::new(db.conn());
        let token = repo.get_valid_token(&req.refresh_token)
            .map_err(|_| ApiError::internal("Database error"))?
            .ok_or_else(|| ApiError::unauthorized("Invalid or expired refresh token"))?;
        token.user_id
    };

    // Get user info
    let user = {
        let db = state.db.lock().await;
        let repo = UserRepository::new(&*db);
        repo.get_by_id(user_id)
            .map_err(|_| ApiError::internal("Database error"))?
            .ok_or_else(|| ApiError::unauthorized("User not found"))?
    };

    // Check if user is active
    if !user.is_active {
        return Err(ApiError::forbidden("Account is disabled"));
    }

    // Revoke old refresh token
    {
        let db = state.db.lock().await;
        let repo = RefreshTokenRepository::new(db.conn());
        let _ = repo.revoke(&req.refresh_token);
    }

    // Generate new tokens
    let access_token = state.generate_access_token(user.id, &user.username, &user.role)?;
    let new_refresh_token = state.generate_refresh_token();

    // Store new refresh token
    {
        let db = state.db.lock().await;
        let repo = RefreshTokenRepository::new(db.conn());
        let expires_at = chrono::Utc::now()
            + chrono::Duration::days(state.refresh_token_expiry as i64);
        let new_token = NewRefreshToken {
            user_id: user.id,
            token: new_refresh_token.clone(),
            expires_at: expires_at.format("%Y-%m-%d %H:%M:%S").to_string(),
        };
        repo.create(&new_token)
            .map_err(|e| {
                tracing::error!("Failed to store refresh token: {}", e);
                ApiError::internal("Failed to create session")
            })?;
    }

    let response = RefreshResponse {
        access_token,
        refresh_token: new_refresh_token,
        expires_in: state.access_token_expiry,
    };

    Ok(Json(ApiResponse::new(response)))
}

/// POST /api/auth/register - User registration.
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
    let user = {
        let db = state.db.lock().await;
        let repo = UserRepository::new(&*db);
        let mut new_user = NewUser::new(&req.username, password_hash, &req.nickname);
        if let Some(ref email) = req.email {
            new_user = new_user.with_email(email);
        }
        repo.create(&new_user)
            .map_err(|e| {
                if e.to_string().contains("UNIQUE") {
                    ApiError::conflict("Username already exists")
                } else {
                    tracing::error!("User creation failed: {}", e);
                    ApiError::internal("Failed to create user")
                }
            })?
    };

    // Generate tokens
    let access_token = state.generate_access_token(user.id, &user.username, &user.role)?;
    let refresh_token = state.generate_refresh_token();

    // Store refresh token in database
    {
        let db = state.db.lock().await;
        let repo = RefreshTokenRepository::new(db.conn());
        let expires_at = chrono::Utc::now()
            + chrono::Duration::days(state.refresh_token_expiry as i64);
        let new_token = NewRefreshToken {
            user_id: user.id,
            token: refresh_token.clone(),
            expires_at: expires_at.format("%Y-%m-%d %H:%M:%S").to_string(),
        };
        repo.create(&new_token)
            .map_err(|e| {
                tracing::error!("Failed to store refresh token: {}", e);
                ApiError::internal("Failed to create session")
            })?;
    }

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
pub async fn me(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser,
) -> Result<Json<ApiResponse<MeResponse>>, ApiError> {
    // Get user from database
    let user = {
        let db = state.db.lock().await;
        let repo = UserRepository::new(&*db);
        repo.get_by_id(claims.sub)
            .map_err(|_| ApiError::internal("Database error"))?
            .ok_or_else(|| ApiError::not_found("User not found"))?
    };

    // Get unread mail count
    let unread_count = {
        let db = state.db.lock().await;
        MailRepository::count_unread(db.conn(), claims.sub).unwrap_or(0)
    };

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
