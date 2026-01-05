//! Web server for HOBBS.

use std::net::SocketAddr;
use std::sync::Arc;

use tokio::net::TcpListener;
use tokio::sync::Mutex;

use crate::chat::ChatRoomManager;
use crate::config::{FilesConfig, WebConfig};
use crate::file::FileStorage;
use crate::Database;

use super::handlers::{AppState, SharedDatabase};
use super::middleware::JwtState;
use super::router::{create_health_router, create_router};

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
    pub fn new(config: &WebConfig, db: SharedDatabase, files_config: Option<&FilesConfig>) -> Self {
        let addr = format!("{}:{}", config.host, config.port)
            .parse()
            .expect("Invalid web server address");

        let mut app_state = AppState::new(
            db,
            &config.jwt_secret,
            config.jwt_access_token_expiry_secs,
            config.jwt_refresh_token_expiry_days,
        );

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
        Self::new(config, Arc::new(Mutex::new(db)), None)
    }

    /// Create a new web server from a raw Database with files config.
    pub fn from_database_with_files(
        config: &WebConfig,
        db: Database,
        files_config: &FilesConfig,
    ) -> Self {
        Self::new(config, Arc::new(Mutex::new(db)), Some(files_config))
    }

    /// Get the server address.
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    /// Run the web server.
    pub async fn run(self) -> Result<(), std::io::Error> {
        let router = create_router(
            self.app_state,
            self.jwt_state,
            self.chat_manager,
            &self.web_config,
        )
        .merge(create_health_router());

        let listener = TcpListener::bind(self.addr).await?;
        let local_addr = listener.local_addr()?;

        tracing::info!("Web server listening on http://{}", local_addr);

        axum::serve(listener, router).await
    }

    /// Run the server and return the actual bound address.
    ///
    /// This is useful for testing when binding to port 0.
    pub async fn run_with_addr(self) -> Result<SocketAddr, std::io::Error> {
        let router = create_router(
            self.app_state,
            self.jwt_state,
            self.chat_manager,
            &self.web_config,
        )
        .merge(create_health_router());

        let listener = TcpListener::bind(self.addr).await?;
        let local_addr = listener.local_addr()?;

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

    #[test]
    fn test_web_server_new() {
        let config = create_test_config();
        let db = Database::open_in_memory().unwrap();

        let server = WebServer::from_database(&config, db);
        assert_eq!(server.addr.ip().to_string(), "127.0.0.1");
    }

    #[tokio::test]
    async fn test_web_server_run() {
        let config = create_test_config();
        let db = Database::open_in_memory().unwrap();

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
