//! JWT authentication middleware.

use axum::{
    body::Body,
    extract::FromRequestParts,
    http::{header::AUTHORIZATION, request::Parts, Request},
    middleware::Next,
    response::Response,
};
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::web::error::ApiError;

/// JWT claims structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtClaims {
    /// Subject (user ID).
    pub sub: i64,
    /// Username.
    pub username: String,
    /// User role.
    pub role: String,
    /// Issued at timestamp.
    pub iat: u64,
    /// Expiration timestamp.
    pub exp: u64,
    /// JWT ID (unique identifier).
    pub jti: String,
}

/// Application state for JWT authentication.
#[derive(Clone)]
pub struct JwtState {
    /// Decoding key for JWT verification.
    pub decoding_key: DecodingKey,
    /// Validation settings.
    pub validation: Validation,
}

impl JwtState {
    /// Create a new JWT state from a secret key.
    pub fn new(secret: &str) -> Self {
        let decoding_key = DecodingKey::from_secret(secret.as_bytes());
        let mut validation = Validation::default();
        validation.validate_exp = true;

        Self {
            decoding_key,
            validation,
        }
    }
}

/// Extractor for authenticated users.
///
/// Use this extractor to require authentication for a handler.
/// The handler will receive the JWT claims if the token is valid.
#[derive(Debug, Clone)]
pub struct AuthUser(pub JwtClaims);

impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = ApiError;

    fn from_request_parts<'life0, 'life1, 'async_trait>(
        parts: &'life0 mut Parts,
        _state: &'life1 S,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Self, Self::Rejection>> + Send + 'async_trait>,
    >
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(async move {
            // Try to get token from Authorization header first
            let token = if let Some(auth_header) = parts
                .headers
                .get(AUTHORIZATION)
                .and_then(|value| value.to_str().ok())
            {
                // Check Bearer prefix
                auth_header
                    .strip_prefix("Bearer ")
                    .map(|t| t.to_string())
            } else {
                None
            };

            // If no header token, try query parameter (for file downloads)
            let token = match token {
                Some(t) => t,
                None => {
                    // Parse query string for token parameter
                    let query = parts.uri.query().unwrap_or("");
                    query
                        .split('&')
                        .find_map(|pair| {
                            let mut parts = pair.splitn(2, '=');
                            let key = parts.next()?;
                            let value = parts.next()?;
                            if key == "token" {
                                // URL decode the token
                                urlencoding::decode(value).ok().map(|s| s.into_owned())
                            } else {
                                None
                            }
                        })
                        .ok_or_else(|| ApiError::unauthorized("Missing authorization"))?
                }
            };

            // Get JWT state from extensions (set by middleware)
            let jwt_state = parts
                .extensions
                .get::<Arc<JwtState>>()
                .ok_or_else(|| ApiError::internal("JWT state not configured"))?;

            // Decode and validate the token
            let token_data =
                decode::<JwtClaims>(&token, &jwt_state.decoding_key, &jwt_state.validation)
                    .map_err(|e| {
                        tracing::debug!("JWT validation failed: {}", e);
                        ApiError::unauthorized("Invalid or expired token")
                    })?;

            Ok(AuthUser(token_data.claims))
        })
    }
}

/// Optional authentication extractor.
///
/// Similar to AuthUser but doesn't fail if no token is provided.
#[derive(Debug, Clone)]
pub struct OptionalAuthUser(pub Option<JwtClaims>);

impl<S> FromRequestParts<S> for OptionalAuthUser
where
    S: Send + Sync,
{
    type Rejection = ApiError;

    fn from_request_parts<'life0, 'life1, 'async_trait>(
        parts: &'life0 mut Parts,
        _state: &'life1 S,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Self, Self::Rejection>> + Send + 'async_trait>,
    >
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(async move {
            // Get the Authorization header
            let auth_header = match parts
                .headers
                .get(AUTHORIZATION)
                .and_then(|v| v.to_str().ok())
            {
                Some(h) => h,
                None => return Ok(OptionalAuthUser(None)),
            };

            // Check Bearer prefix
            let token = match auth_header.strip_prefix("Bearer ") {
                Some(t) => t,
                None => return Ok(OptionalAuthUser(None)),
            };

            // Get JWT state from extensions
            let jwt_state = match parts.extensions.get::<Arc<JwtState>>() {
                Some(s) => s,
                None => return Ok(OptionalAuthUser(None)),
            };

            // Decode and validate the token
            match decode::<JwtClaims>(token, &jwt_state.decoding_key, &jwt_state.validation) {
                Ok(token_data) => Ok(OptionalAuthUser(Some(token_data.claims))),
                Err(_) => Ok(OptionalAuthUser(None)),
            }
        })
    }
}

/// Middleware function to inject JWT state into request extensions.
pub async fn jwt_auth(
    jwt_state: Arc<JwtState>,
    mut request: Request<Body>,
    next: Next,
) -> Response {
    request.extensions_mut().insert(jwt_state);
    next.run(request).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use jsonwebtoken::{encode, EncodingKey, Header};

    fn create_test_token(secret: &str, claims: &JwtClaims) -> String {
        encode(
            &Header::default(),
            claims,
            &EncodingKey::from_secret(secret.as_bytes()),
        )
        .unwrap()
    }

    #[test]
    fn test_jwt_state_new() {
        let state = JwtState::new("test-secret");
        assert!(state.validation.validate_exp);
    }

    #[test]
    fn test_create_and_verify_token() {
        let secret = "test-secret";
        let state = JwtState::new(secret);

        let claims = JwtClaims {
            sub: 1,
            username: "testuser".to_string(),
            role: "member".to_string(),
            iat: chrono::Utc::now().timestamp() as u64,
            exp: (chrono::Utc::now().timestamp() + 3600) as u64,
            jti: uuid::Uuid::new_v4().to_string(),
        };

        let token = create_test_token(secret, &claims);

        let decoded = decode::<JwtClaims>(&token, &state.decoding_key, &state.validation).unwrap();
        assert_eq!(decoded.claims.sub, 1);
        assert_eq!(decoded.claims.username, "testuser");
        assert_eq!(decoded.claims.role, "member");
    }

    #[test]
    fn test_expired_token() {
        let secret = "test-secret";
        let state = JwtState::new(secret);

        let claims = JwtClaims {
            sub: 1,
            username: "testuser".to_string(),
            role: "member".to_string(),
            iat: (chrono::Utc::now().timestamp() - 7200) as u64,
            exp: (chrono::Utc::now().timestamp() - 3600) as u64, // Expired 1 hour ago
            jti: uuid::Uuid::new_v4().to_string(),
        };

        let token = create_test_token(secret, &claims);

        let result = decode::<JwtClaims>(&token, &state.decoding_key, &state.validation);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_secret() {
        let claims = JwtClaims {
            sub: 1,
            username: "testuser".to_string(),
            role: "member".to_string(),
            iat: chrono::Utc::now().timestamp() as u64,
            exp: (chrono::Utc::now().timestamp() + 3600) as u64,
            jti: uuid::Uuid::new_v4().to_string(),
        };

        let token = create_test_token("secret1", &claims);
        let state = JwtState::new("secret2"); // Different secret

        let result = decode::<JwtClaims>(&token, &state.decoding_key, &state.validation);
        assert!(result.is_err());
    }
}
