//! Rate limiting middleware.

use axum::{
    body::Body,
    extract::ConnectInfo,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use governor::{
    clock::DefaultClock,
    state::{InMemoryState, NotKeyed},
    Quota, RateLimiter,
};
use std::{
    collections::HashMap,
    net::SocketAddr,
    num::NonZeroU32,
    sync::{Arc, RwLock},
    time::Duration,
};

/// Per-IP rate limiter using Governor.
pub type IpRateLimiter = RateLimiter<NotKeyed, InMemoryState, DefaultClock>;

/// State for rate limiting.
#[derive(Clone)]
pub struct RateLimitState {
    /// Per-IP rate limiters for login endpoint.
    login_limiters: Arc<RwLock<HashMap<String, Arc<IpRateLimiter>>>>,
    /// Per-IP rate limiters for general API.
    api_limiters: Arc<RwLock<HashMap<String, Arc<IpRateLimiter>>>>,
    /// Login rate limit (requests per minute).
    login_rate_limit: u32,
    /// API rate limit (requests per minute).
    api_rate_limit: u32,
}

impl RateLimitState {
    /// Create a new rate limit state.
    pub fn new(login_rate_limit: u32, api_rate_limit: u32) -> Self {
        Self {
            login_limiters: Arc::new(RwLock::new(HashMap::new())),
            api_limiters: Arc::new(RwLock::new(HashMap::new())),
            login_rate_limit,
            api_rate_limit,
        }
    }

    /// Get or create a rate limiter for the given IP.
    fn get_or_create_limiter(
        limiters: &RwLock<HashMap<String, Arc<IpRateLimiter>>>,
        ip: &str,
        requests_per_minute: u32,
    ) -> Arc<IpRateLimiter> {
        // Try read lock first
        {
            let read_guard = limiters.read().unwrap();
            if let Some(limiter) = read_guard.get(ip) {
                return limiter.clone();
            }
        }

        // Create new limiter
        let mut write_guard = limiters.write().unwrap();

        // Double-check after acquiring write lock
        if let Some(limiter) = write_guard.get(ip) {
            return limiter.clone();
        }

        let quota = Quota::per_minute(NonZeroU32::new(requests_per_minute).unwrap_or(NonZeroU32::MIN));
        let limiter = Arc::new(RateLimiter::direct(quota));
        write_guard.insert(ip.to_string(), limiter.clone());
        limiter
    }

    /// Check if a request is allowed for login endpoint.
    pub fn check_login(&self, ip: &str) -> bool {
        let limiter = Self::get_or_create_limiter(&self.login_limiters, ip, self.login_rate_limit);
        limiter.check().is_ok()
    }

    /// Check if a request is allowed for general API.
    pub fn check_api(&self, ip: &str) -> bool {
        let limiter = Self::get_or_create_limiter(&self.api_limiters, ip, self.api_rate_limit);
        limiter.check().is_ok()
    }

    /// Cleanup old entries (call periodically).
    pub fn cleanup(&self) {
        // Remove limiters that haven't been used for a while
        // Governor handles this internally via reference counting
        let mut login_guard = self.login_limiters.write().unwrap();
        login_guard.retain(|_, v| Arc::strong_count(v) > 1);

        let mut api_guard = self.api_limiters.write().unwrap();
        api_guard.retain(|_, v| Arc::strong_count(v) > 1);
    }

    /// Start a background task to periodically clean up old entries.
    pub fn start_cleanup_task(self: Arc<Self>) {
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(300)).await; // Every 5 minutes
                self.cleanup();
            }
        });
    }
}

/// Extract client IP from request.
fn get_client_ip(req: &Request<Body>) -> String {
    // Try X-Forwarded-For header first (for reverse proxy)
    if let Some(forwarded) = req
        .headers()
        .get("X-Forwarded-For")
        .and_then(|v| v.to_str().ok())
    {
        // Take the first IP in the chain
        if let Some(ip) = forwarded.split(',').next() {
            return ip.trim().to_string();
        }
    }

    // Try X-Real-IP header
    if let Some(real_ip) = req
        .headers()
        .get("X-Real-IP")
        .and_then(|v| v.to_str().ok())
    {
        return real_ip.to_string();
    }

    // Fall back to connection info
    if let Some(ConnectInfo(addr)) = req.extensions().get::<ConnectInfo<SocketAddr>>() {
        return addr.ip().to_string();
    }

    // Default to unknown
    "unknown".to_string()
}

/// Rate limiting middleware for login endpoint.
pub async fn login_rate_limit(
    state: Arc<RateLimitState>,
    req: Request<Body>,
    next: Next,
) -> Response {
    let ip = get_client_ip(&req);

    if !state.check_login(&ip) {
        tracing::warn!(ip = %ip, "Login rate limit exceeded");
        return (
            StatusCode::TOO_MANY_REQUESTS,
            "Too many login attempts. Please try again later.",
        )
            .into_response();
    }

    next.run(req).await
}

/// Rate limiting middleware for general API.
pub async fn api_rate_limit(
    state: Arc<RateLimitState>,
    req: Request<Body>,
    next: Next,
) -> Response {
    let ip = get_client_ip(&req);

    if !state.check_api(&ip) {
        tracing::warn!(ip = %ip, "API rate limit exceeded");
        return (
            StatusCode::TOO_MANY_REQUESTS,
            "Too many requests. Please try again later.",
        )
            .into_response();
    }

    next.run(req).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limit_state_new() {
        let state = RateLimitState::new(5, 100);
        assert_eq!(state.login_rate_limit, 5);
        assert_eq!(state.api_rate_limit, 100);
    }

    #[test]
    fn test_login_rate_limit() {
        let state = RateLimitState::new(3, 100);

        // First 3 requests should succeed
        assert!(state.check_login("127.0.0.1"));
        assert!(state.check_login("127.0.0.1"));
        assert!(state.check_login("127.0.0.1"));

        // 4th request should fail
        assert!(!state.check_login("127.0.0.1"));

        // Different IP should work
        assert!(state.check_login("192.168.1.1"));
    }

    #[test]
    fn test_api_rate_limit() {
        let state = RateLimitState::new(5, 3);

        // First 3 requests should succeed
        assert!(state.check_api("127.0.0.1"));
        assert!(state.check_api("127.0.0.1"));
        assert!(state.check_api("127.0.0.1"));

        // 4th request should fail
        assert!(!state.check_api("127.0.0.1"));
    }
}
