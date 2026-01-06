//! Application module.
//!
//! Provides the main application logic and session handling.

mod menu;
pub mod screens;
mod session_handler;

pub use menu::{MenuAction, MenuError};
pub use screens::ScreenResult;
pub use session_handler::SessionHandler;

use std::sync::Arc;

use crate::chat::ChatRoomManager;
use crate::config::Config;
use crate::db::Database;
use crate::error::Result;
use crate::i18n::I18nManager;
use crate::rate_limit::{RateLimitConfig, RateLimiters};
use crate::server::{SessionManager, TelnetSession};
use crate::template::TemplateLoader;
use crate::terminal::TerminalProfile;

/// Main application that manages BBS functionality.
pub struct Application {
    /// Database connection.
    db: Arc<Database>,
    /// Application configuration.
    config: Arc<Config>,
    /// Internationalization manager.
    i18n_manager: Arc<I18nManager>,
    /// Template loader.
    template_loader: Arc<TemplateLoader>,
    /// Session manager for tracking connected users.
    session_manager: Arc<SessionManager>,
    /// Chat room manager.
    chat_manager: Arc<ChatRoomManager>,
    /// Rate limiters for user actions.
    rate_limiters: Arc<RateLimiters>,
}

impl Application {
    /// Create a new application instance.
    pub fn new(
        db: Arc<Database>,
        config: Arc<Config>,
        i18n_manager: Arc<I18nManager>,
        template_loader: Arc<TemplateLoader>,
        session_manager: Arc<SessionManager>,
        chat_manager: Arc<ChatRoomManager>,
    ) -> Self {
        // Create rate limiters from config
        let rate_limiters = Arc::new(RateLimiters::with_config(
            RateLimitConfig::new(config.rate_limits.post_per_minute, 60),
            RateLimitConfig::new(config.rate_limits.chat_per_10_seconds, 10),
            RateLimitConfig::new(config.rate_limits.mail_per_minute, 60),
        ));

        Self {
            db,
            config,
            i18n_manager,
            template_loader,
            session_manager,
            chat_manager,
            rate_limiters,
        }
    }

    /// Get the database.
    pub fn db(&self) -> &Arc<Database> {
        &self.db
    }

    /// Get the configuration.
    pub fn config(&self) -> &Arc<Config> {
        &self.config
    }

    /// Get the i18n manager.
    pub fn i18n_manager(&self) -> &Arc<I18nManager> {
        &self.i18n_manager
    }

    /// Get the template loader.
    pub fn template_loader(&self) -> &Arc<TemplateLoader> {
        &self.template_loader
    }

    /// Get the session manager.
    pub fn session_manager(&self) -> &Arc<SessionManager> {
        &self.session_manager
    }

    /// Get the chat room manager.
    pub fn chat_manager(&self) -> &Arc<ChatRoomManager> {
        &self.chat_manager
    }

    /// Get the rate limiters.
    pub fn rate_limiters(&self) -> &Arc<RateLimiters> {
        &self.rate_limiters
    }

    /// Create a session handler for a new connection.
    ///
    /// Uses the default terminal profile from config.
    pub fn create_session_handler(&self) -> SessionHandler {
        SessionHandler::new(
            Arc::clone(&self.db),
            Arc::clone(&self.config),
            Arc::clone(&self.i18n_manager),
            Arc::clone(&self.template_loader),
            Arc::clone(&self.session_manager),
            Arc::clone(&self.chat_manager),
            Arc::clone(&self.rate_limiters),
        )
    }

    /// Create a session handler with a specific terminal profile.
    pub fn create_session_handler_with_profile(&self, profile: TerminalProfile) -> SessionHandler {
        SessionHandler::with_profile(
            Arc::clone(&self.db),
            Arc::clone(&self.config),
            Arc::clone(&self.i18n_manager),
            Arc::clone(&self.template_loader),
            Arc::clone(&self.session_manager),
            Arc::clone(&self.chat_manager),
            Arc::clone(&self.rate_limiters),
            profile,
        )
    }

    /// Run a session.
    ///
    /// This is the main entry point for handling a connected client.
    /// Uses the default terminal profile from config.
    pub async fn run_session(&self, session: &mut TelnetSession) -> Result<()> {
        let mut handler = self.create_session_handler();
        handler.run(session).await
    }
}

impl Clone for Application {
    fn clone(&self) -> Self {
        Self {
            db: Arc::clone(&self.db),
            config: Arc::clone(&self.config),
            i18n_manager: Arc::clone(&self.i18n_manager),
            template_loader: Arc::clone(&self.template_loader),
            session_manager: Arc::clone(&self.session_manager),
            chat_manager: Arc::clone(&self.chat_manager),
            rate_limiters: Arc::clone(&self.rate_limiters),
        }
    }
}

#[cfg(test)]
mod tests {
    // Tests will be added as the module is developed
}
