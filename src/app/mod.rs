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

use crate::config::Config;
use crate::db::Database;
use crate::error::Result;
use crate::i18n::I18nManager;
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
}

impl Application {
    /// Create a new application instance.
    pub fn new(
        db: Arc<Database>,
        config: Arc<Config>,
        i18n_manager: Arc<I18nManager>,
        template_loader: Arc<TemplateLoader>,
        session_manager: Arc<SessionManager>,
    ) -> Self {
        Self {
            db,
            config,
            i18n_manager,
            template_loader,
            session_manager,
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

    /// Create a session handler for a new connection.
    pub fn create_session_handler(&self, profile: TerminalProfile) -> SessionHandler {
        SessionHandler::new(
            Arc::clone(&self.db),
            Arc::clone(&self.config),
            Arc::clone(&self.i18n_manager),
            Arc::clone(&self.template_loader),
            Arc::clone(&self.session_manager),
            profile,
        )
    }

    /// Run a session.
    ///
    /// This is the main entry point for handling a connected client.
    pub async fn run_session(&self, session: &mut TelnetSession) -> Result<()> {
        let profile = TerminalProfile::standard(); // TODO: Allow profile selection
        let mut handler = self.create_session_handler(profile);
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
        }
    }
}

#[cfg(test)]
mod tests {
    // Tests will be added as the module is developed
}
