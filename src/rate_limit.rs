//! Action-based rate limiting for user actions.
//!
//! Provides rate limiting for specific user actions like posting,
//! chat messages, and sending mail.

use std::collections::HashMap;
use std::sync::RwLock;
use std::time::{Duration, Instant};

/// Configuration for rate limiting.
#[derive(Debug, Clone, Copy)]
pub struct RateLimitConfig {
    /// Maximum actions allowed in the time window.
    pub max_actions: u32,
    /// Time window for counting actions.
    pub window: Duration,
}

impl RateLimitConfig {
    /// Create a new rate limit configuration.
    pub fn new(max_actions: u32, window_secs: u64) -> Self {
        Self {
            max_actions,
            window: Duration::from_secs(window_secs),
        }
    }
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_actions: 10,
            window: Duration::from_secs(60),
        }
    }
}

/// Result of a rate limit check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RateLimitResult {
    /// Action is allowed.
    Allowed,
    /// Action is denied due to rate limit.
    Denied {
        /// Time until the rate limit resets.
        retry_after: Duration,
    },
}

impl RateLimitResult {
    /// Check if the action is allowed.
    pub fn is_allowed(&self) -> bool {
        matches!(self, RateLimitResult::Allowed)
    }
}

/// Tracks action timestamps for a single user.
#[derive(Debug)]
struct UserActions {
    /// Timestamps of recent actions.
    timestamps: Vec<Instant>,
}

impl UserActions {
    fn new() -> Self {
        Self {
            timestamps: Vec::new(),
        }
    }

    /// Clean up old timestamps outside the window.
    fn cleanup(&mut self, window: Duration) {
        let cutoff = Instant::now() - window;
        self.timestamps.retain(|&t| t > cutoff);
    }

    /// Count actions within the window.
    fn count_in_window(&self, window: Duration) -> usize {
        let cutoff = Instant::now() - window;
        self.timestamps.iter().filter(|&&t| t > cutoff).count()
    }

    /// Get the oldest timestamp in the window.
    fn oldest_in_window(&self, window: Duration) -> Option<Instant> {
        let cutoff = Instant::now() - window;
        self.timestamps
            .iter()
            .filter(|&&t| t > cutoff)
            .min()
            .copied()
    }

    /// Record a new action.
    fn record(&mut self) {
        self.timestamps.push(Instant::now());
    }
}

/// Action-based rate limiter.
///
/// Tracks user actions and enforces rate limits per user.
///
/// # Example
///
/// ```
/// use hobbs::rate_limit::{ActionRateLimiter, RateLimitConfig};
/// use std::time::Duration;
///
/// let config = RateLimitConfig::new(5, 60); // 5 actions per minute
/// let limiter = ActionRateLimiter::new(config);
///
/// // Check if user can perform action
/// let result = limiter.check(1);
/// assert!(result.is_allowed());
///
/// // Record the action
/// limiter.record(1);
/// ```
#[derive(Debug)]
pub struct ActionRateLimiter {
    /// Rate limit configuration.
    config: RateLimitConfig,
    /// Per-user action tracking.
    users: RwLock<HashMap<i64, UserActions>>,
}

impl ActionRateLimiter {
    /// Create a new rate limiter with the given configuration.
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            config,
            users: RwLock::new(HashMap::new()),
        }
    }

    /// Check if a user can perform an action.
    ///
    /// This does not record the action - call `record()` after the action succeeds.
    pub fn check(&self, user_id: i64) -> RateLimitResult {
        let users = self.users.read().unwrap();

        if let Some(actions) = users.get(&user_id) {
            let count = actions.count_in_window(self.config.window);

            if count >= self.config.max_actions as usize {
                // Calculate retry_after based on oldest action in window
                if let Some(oldest) = actions.oldest_in_window(self.config.window) {
                    let elapsed = oldest.elapsed();
                    let retry_after = if elapsed < self.config.window {
                        self.config.window - elapsed
                    } else {
                        Duration::ZERO
                    };
                    return RateLimitResult::Denied { retry_after };
                }
            }
        }

        RateLimitResult::Allowed
    }

    /// Record a successful action for a user.
    ///
    /// Call this after the action has been successfully performed.
    pub fn record(&self, user_id: i64) {
        let mut users = self.users.write().unwrap();
        let actions = users.entry(user_id).or_insert_with(UserActions::new);
        actions.cleanup(self.config.window);
        actions.record();
    }

    /// Check and record in one operation.
    ///
    /// Returns `Allowed` and records the action, or returns `Denied` without recording.
    pub fn check_and_record(&self, user_id: i64) -> RateLimitResult {
        let mut users = self.users.write().unwrap();
        let actions = users.entry(user_id).or_insert_with(UserActions::new);

        // Cleanup old entries
        actions.cleanup(self.config.window);

        let count = actions.count_in_window(self.config.window);

        if count >= self.config.max_actions as usize {
            if let Some(oldest) = actions.oldest_in_window(self.config.window) {
                let elapsed = oldest.elapsed();
                let retry_after = if elapsed < self.config.window {
                    self.config.window - elapsed
                } else {
                    Duration::ZERO
                };
                return RateLimitResult::Denied { retry_after };
            }
        }

        actions.record();
        RateLimitResult::Allowed
    }

    /// Cleanup old entries for all users.
    ///
    /// Call this periodically to free memory.
    pub fn cleanup(&self) {
        let mut users = self.users.write().unwrap();

        // Cleanup each user's timestamps
        for actions in users.values_mut() {
            actions.cleanup(self.config.window);
        }

        // Remove users with no recent actions
        users.retain(|_, actions| !actions.timestamps.is_empty());
    }

    /// Get the number of remaining actions for a user.
    pub fn remaining(&self, user_id: i64) -> u32 {
        let users = self.users.read().unwrap();

        if let Some(actions) = users.get(&user_id) {
            let count = actions.count_in_window(self.config.window);
            self.config
                .max_actions
                .saturating_sub(count as u32)
        } else {
            self.config.max_actions
        }
    }
}

/// Collection of rate limiters for different actions.
#[derive(Debug)]
pub struct RateLimiters {
    /// Rate limiter for board posts.
    pub post: ActionRateLimiter,
    /// Rate limiter for chat messages.
    pub chat: ActionRateLimiter,
    /// Rate limiter for mail sending.
    pub mail: ActionRateLimiter,
}

impl RateLimiters {
    /// Create rate limiters with default configurations.
    ///
    /// Defaults:
    /// - Posts: 5 per minute
    /// - Chat: 10 per 10 seconds
    /// - Mail: 3 per minute
    pub fn new() -> Self {
        Self {
            post: ActionRateLimiter::new(RateLimitConfig::new(5, 60)),
            chat: ActionRateLimiter::new(RateLimitConfig::new(10, 10)),
            mail: ActionRateLimiter::new(RateLimitConfig::new(3, 60)),
        }
    }

    /// Create rate limiters with custom configurations.
    pub fn with_config(
        post_config: RateLimitConfig,
        chat_config: RateLimitConfig,
        mail_config: RateLimitConfig,
    ) -> Self {
        Self {
            post: ActionRateLimiter::new(post_config),
            chat: ActionRateLimiter::new(chat_config),
            mail: ActionRateLimiter::new(mail_config),
        }
    }

    /// Cleanup all rate limiters.
    pub fn cleanup(&self) {
        self.post.cleanup();
        self.chat.cleanup();
        self.mail.cleanup();
    }
}

impl Default for RateLimiters {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limit_config() {
        let config = RateLimitConfig::new(5, 60);
        assert_eq!(config.max_actions, 5);
        assert_eq!(config.window, Duration::from_secs(60));
    }

    #[test]
    fn test_rate_limiter_allows_under_limit() {
        let config = RateLimitConfig::new(3, 60);
        let limiter = ActionRateLimiter::new(config);

        // First 3 actions should be allowed
        assert!(limiter.check_and_record(1).is_allowed());
        assert!(limiter.check_and_record(1).is_allowed());
        assert!(limiter.check_and_record(1).is_allowed());
    }

    #[test]
    fn test_rate_limiter_denies_over_limit() {
        let config = RateLimitConfig::new(2, 60);
        let limiter = ActionRateLimiter::new(config);

        // First 2 actions allowed
        assert!(limiter.check_and_record(1).is_allowed());
        assert!(limiter.check_and_record(1).is_allowed());

        // 3rd action denied
        let result = limiter.check_and_record(1);
        assert!(!result.is_allowed());

        match result {
            RateLimitResult::Denied { retry_after } => {
                assert!(retry_after <= Duration::from_secs(60));
            }
            _ => panic!("Expected Denied"),
        }
    }

    #[test]
    fn test_rate_limiter_separate_users() {
        let config = RateLimitConfig::new(2, 60);
        let limiter = ActionRateLimiter::new(config);

        // User 1 uses their limit
        assert!(limiter.check_and_record(1).is_allowed());
        assert!(limiter.check_and_record(1).is_allowed());
        assert!(!limiter.check_and_record(1).is_allowed());

        // User 2 should still be able to act
        assert!(limiter.check_and_record(2).is_allowed());
        assert!(limiter.check_and_record(2).is_allowed());
    }

    #[test]
    fn test_remaining_count() {
        let config = RateLimitConfig::new(5, 60);
        let limiter = ActionRateLimiter::new(config);

        assert_eq!(limiter.remaining(1), 5);

        limiter.record(1);
        assert_eq!(limiter.remaining(1), 4);

        limiter.record(1);
        limiter.record(1);
        assert_eq!(limiter.remaining(1), 2);
    }

    #[test]
    fn test_rate_limiters_collection() {
        let limiters = RateLimiters::new();

        // Each limiter should work independently
        assert!(limiters.post.check(1).is_allowed());
        assert!(limiters.chat.check(1).is_allowed());
        assert!(limiters.mail.check(1).is_allowed());
    }

    #[test]
    fn test_check_without_record() {
        let config = RateLimitConfig::new(2, 60);
        let limiter = ActionRateLimiter::new(config);

        // Check doesn't record
        assert!(limiter.check(1).is_allowed());
        assert!(limiter.check(1).is_allowed());
        assert!(limiter.check(1).is_allowed());

        // Still at 2 remaining since we didn't record
        assert_eq!(limiter.remaining(1), 2);
    }
}
