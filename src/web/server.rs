//! Web server for HOBBS.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use tokio::net::TcpListener;
use tower_http::compression::CompressionLayer;

use crate::chat::ChatRoomManager;
use crate::config::{BbsConfig, FilesConfig, WebConfig};
use crate::db::{OneTimeTokenRepository, RefreshTokenRepository};
use crate::file::FileStorage;
use crate::Database;

use super::handlers::{AppState, SharedDatabase};
use super::middleware::JwtState;
use super::router::{
    create_health_router, create_router, create_static_router, create_swagger_router,
};

/// Web server for the API.
pub struct WebServer {
    /// Server address.
    addr: SocketAddr,
    /// Application state.
    app_state: Arc<AppState>,
    /// JWT state.
    jwt_state: Arc<JwtState>,
    /// Web configuration.
    web_config: WebConfig,
    /// Chat room manager.
    chat_manager: Option<Arc<ChatRoomManager>>,
}

impl WebServer {
    /// Create a new web server.
    pub fn new(
        config: &WebConfig,
        db: SharedDatabase,
        files_config: Option<&FilesConfig>,
        bbs_config: Option<&BbsConfig>,
        telnet_enabled: bool,
    ) -> Self {
        let addr = format!("{}:{}", config.host, config.port)
            .parse()
            .expect("Invalid web server address");

        let mut app_state = AppState::new(
            db,
            &config.jwt_secret,
            config.jwt_access_token_expiry_secs,
            config.jwt_refresh_token_expiry_days,
        )
        .with_telnet_enabled(telnet_enabled);

        // Apply BBS config if provided
        if let Some(bbs) = bbs_config {
            app_state = app_state.with_bbs_config(&bbs.name, &bbs.description, &bbs.sysop_name);
        }

        // Initialize file storage if files config is provided
        if let Some(files) = files_config {
            match FileStorage::new(&files.storage_path) {
                Ok(storage) => {
                    app_state = app_state.with_file_storage(storage, files.max_upload_size_mb);
                    tracing::info!("File storage initialized at: {}", files.storage_path);
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to initialize file storage: {}. File API will be disabled.",
                        e
                    );
                }
            }
        }

        let jwt_state = Arc::new(JwtState::new(&config.jwt_secret));

        Self {
            addr,
            app_state: Arc::new(app_state),
            jwt_state,
            web_config: config.clone(),
            chat_manager: None,
        }
    }

    /// Set the chat room manager.
    pub fn with_chat_manager(mut self, chat_manager: Arc<ChatRoomManager>) -> Self {
        self.chat_manager = Some(chat_manager);
        self
    }

    /// Create a new web server from a raw Database.
    pub fn from_database(config: &WebConfig, db: Database) -> Self {
        Self::new(config, Arc::new(db), None, None, true)
    }

    /// Create a new web server from a raw Database with files config.
    pub fn from_database_with_files(
        config: &WebConfig,
        db: Database,
        files_config: &FilesConfig,
    ) -> Self {
        Self::new(config, Arc::new(db), Some(files_config), None, true)
    }

    /// Create a new web server from a raw Database with files and BBS config.
    pub fn from_database_with_configs(
        config: &WebConfig,
        db: Database,
        files_config: &FilesConfig,
        bbs_config: &BbsConfig,
        telnet_enabled: bool,
    ) -> Self {
        Self::new(
            config,
            Arc::new(db),
            Some(files_config),
            Some(bbs_config),
            telnet_enabled,
        )
    }

    /// Get the server address.
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    /// Start the token cleanup background task.
    ///
    /// This task runs every hour and removes:
    /// - Expired and revoked refresh tokens
    /// - Expired and used one-time tokens
    fn start_token_cleanup_task(db: SharedDatabase) {
        tokio::spawn(async move {
            // Token cleanup interval: 1 hour
            const CLEANUP_INTERVAL_SECS: u64 = 3600;

            let mut interval = tokio::time::interval(Duration::from_secs(CLEANUP_INTERVAL_SECS));

            // Skip the first immediate tick
            interval.tick().await;

            loop {
                interval.tick().await;

                // Cleanup refresh tokens
                let refresh_repo = RefreshTokenRepository::new(db.pool());
                match refresh_repo.cleanup_expired().await {
                    Ok(count) => {
                        if count > 0 {
                            tracing::info!(
                                deleted_count = count,
                                "Cleaned up expired/revoked refresh tokens"
                            );
                        } else {
                            tracing::debug!("No expired refresh tokens to clean up");
                        }
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, "Failed to cleanup refresh tokens");
                    }
                }

                // Cleanup one-time tokens
                let ott_repo = OneTimeTokenRepository::new(db.pool());
                match ott_repo.cleanup().await {
                    Ok(count) => {
                        if count > 0 {
                            tracing::info!(
                                deleted_count = count,
                                "Cleaned up expired/used one-time tokens"
                            );
                        } else {
                            tracing::debug!("No expired one-time tokens to clean up");
                        }
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, "Failed to cleanup one-time tokens");
                    }
                }
            }
        });
    }

    /// Run the web server.
    pub async fn run(self) -> Result<(), std::io::Error> {
        // Clone db reference before moving app_state to router
        let db = self.app_state.db.clone();

        let mut router = create_router(
            self.app_state,
            self.jwt_state,
            self.chat_manager,
            &self.web_config,
        )
        .merge(create_health_router())
        .merge(create_swagger_router());

        // Add static file serving if enabled
        if self.web_config.serve_static {
            if let Some(static_router) = create_static_router(&self.web_config.static_path) {
                router = router.merge(static_router);
            }
        }

        // Add gzip compression layer
        let router = router.layer(CompressionLayer::new());

        let listener = TcpListener::bind(self.addr).await?;
        let local_addr = listener.local_addr()?;

        // Start token cleanup background task after successful bind
        Self::start_token_cleanup_task(db);
        tracing::info!("Token cleanup task started (runs every hour)");

        tracing::info!("Web server listening on http://{}", local_addr);

        axum::serve(listener, router).await
    }

    /// Run the server and return the actual bound address.
    ///
    /// This is useful for testing when binding to port 0.
    pub async fn run_with_addr(self) -> Result<SocketAddr, std::io::Error> {
        // Clone db reference before moving app_state to router
        let db = self.app_state.db.clone();

        let mut router = create_router(
            self.app_state,
            self.jwt_state,
            self.chat_manager,
            &self.web_config,
        )
        .merge(create_health_router())
        .merge(create_swagger_router());

        // Add static file serving if enabled
        if self.web_config.serve_static {
            if let Some(static_router) = create_static_router(&self.web_config.static_path) {
                router = router.merge(static_router);
            }
        }

        // Add gzip compression layer
        let router = router.layer(CompressionLayer::new());

        let listener = TcpListener::bind(self.addr).await?;
        let local_addr = listener.local_addr()?;

        // Start token cleanup background task after successful bind
        Self::start_token_cleanup_task(db);
        tracing::info!("Token cleanup task started (runs every hour)");

        tracing::info!("Web server listening on http://{}", local_addr);

        tokio::spawn(async move {
            if let Err(e) = axum::serve(listener, router).await {
                tracing::error!("Web server error: {}", e);
            }
        });

        Ok(local_addr)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> WebConfig {
        WebConfig {
            enabled: true,
            host: "127.0.0.1".to_string(),
            port: 0, // Use random port
            cors_origins: vec![],
            jwt_secret: "test-secret-key".to_string(),
            jwt_access_token_expiry_secs: 900,
            jwt_refresh_token_expiry_days: 7,
            serve_static: false,
            static_path: "web/dist".to_string(),
            login_rate_limit: 5,
            api_rate_limit: 100,
        }
    }

    #[tokio::test]
    async fn test_web_server_new() {
        let config = create_test_config();
        let db = Database::open_in_memory().await.unwrap();

        let server = WebServer::from_database(&config, db);
        assert_eq!(server.addr.ip().to_string(), "127.0.0.1");
    }

    #[tokio::test]
    async fn test_web_server_run() {
        let config = create_test_config();
        let db = Database::open_in_memory().await.unwrap();

        let server = WebServer::from_database(&config, db);
        let addr = server.run_with_addr().await.unwrap();

        // Test health endpoint
        let client = reqwest::Client::new();
        let resp = client
            .get(format!("http://{}/health", addr))
            .send()
            .await
            .unwrap();

        assert!(resp.status().is_success());
        assert_eq!(resp.text().await.unwrap(), "OK");
    }
}
