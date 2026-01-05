//! Middleware for Web API.

pub mod auth;
pub mod cors;
pub mod rate_limit;
pub mod security;

pub use auth::{jwt_auth, AuthUser, JwtClaims, JwtState, OptionalAuthUser};
pub use cors::create_cors_layer;
pub use rate_limit::{api_rate_limit, login_rate_limit, RateLimitState};
pub use security::security_headers;
