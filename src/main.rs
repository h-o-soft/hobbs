use std::sync::Arc;

use tracing::{error, info};

use hobbs::server::SessionManager;
use hobbs::web::WebServer;
use hobbs::{
    chat::ChatRoomManager, start_rss_updater_with_config, Application, Config, Database,
    HobbsError, I18nManager, TelnetServer, TelnetSession, TemplateLoader,
};

fn main() {
    // Load configuration with environment variable overrides
    let config = match Config::load_with_env("config.toml") {
        Ok(config) => config,
        Err(e) => {
            eprintln!("Failed to load config.toml: {e}");
            eprintln!("Using default configuration.");
            let mut default_config = Config::default();
            default_config.apply_env_overrides();
            default_config
        }
    };

    // Validate configuration
    if let Err(e) = config.validate() {
        eprintln!("Configuration error: {e}");
        std::process::exit(1);
    }

    // Initialize logging
    if let Err(e) = hobbs::logging::init(&config.logging) {
        eprintln!("Failed to initialize logging: {e}");
        // Fall back to console-only logging
        hobbs::logging::init_console_only(&config.logging.level);
    }

    info!("HOBBS - Hobbyist Bulletin Board System");
    info!(
        "Server starting on {}:{}",
        config.server.host, config.server.port
    );

    // Create tokio runtime
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Failed to create tokio runtime");

    // Run the server
    if let Err(e) = rt.block_on(run_server(config)) {
        error!("Server error: {e}");
        std::process::exit(1);
    }
}

async fn run_server(config: Config) -> Result<(), Box<dyn std::error::Error>> {
    // Use LocalSet because TelnetSession/ScreenContext contain non-Send types (Cell)
    let local = tokio::task::LocalSet::new();

    local
        .run_until(async move {
            // Open database with pool configuration
            #[cfg(feature = "sqlite")]
            let db = {
                // Ensure data directory exists for SQLite
                std::fs::create_dir_all("data")?;
                Arc::new(Database::open_with_config(&config.database).await?)
            };
            #[cfg(feature = "postgres")]
            let db = Arc::new(Database::open_with_config(&config.database).await?);

            #[cfg(feature = "sqlite")]
            info!(
                "Database opened: {} (pool_size={}, min_connections={})",
                config.database.path, config.database.pool_size, config.database.min_connections
            );
            #[cfg(feature = "postgres")]
            info!(
                "PostgreSQL database connected (pool_size={}, min_connections={})",
                config.database.pool_size, config.database.min_connections
            );

            // Load I18n
            let i18n_manager = Arc::new(I18nManager::load_all("locales")?);
            info!("I18n loaded");

            // Load templates
            let template_loader = Arc::new(TemplateLoader::new(&config.templates.path));
            info!("Templates loaded from: {}", config.templates.path);

            // Create session manager
            let session_manager = Arc::new(SessionManager::new(config.server.idle_timeout_secs));

            // Create chat room manager
            let chat_manager = Arc::new(ChatRoomManager::with_defaults().await);
            info!("Chat rooms initialized");

            // Create application
            let app = Application::new(
                db,
                Arc::new(config.clone()),
                i18n_manager,
                template_loader,
                session_manager,
                Arc::clone(&chat_manager),
            );

            // Bind server
            let server = TelnetServer::bind(&config.server).await?;
            info!(
                "Server listening on {}:{}",
                config.server.host, config.server.port
            );
            info!("Press Ctrl+C to stop");

            // Start Web server if enabled (runs in separate task with its own DB connection)
            // Web server uses Send-safe types, so tokio::spawn is fine
            if config.web.enabled {
                #[cfg(feature = "sqlite")]
                let web_db = Database::open(&config.database.path).await?;
                #[cfg(feature = "postgres")]
                let web_db = {
                    let url = if !config.database.url.is_empty() {
                        config.database.url.clone()
                    } else {
                        std::env::var("DATABASE_URL")
                            .map_err(|_| HobbsError::Config(
                                "PostgreSQL requires database.url in config or DATABASE_URL environment variable".to_string()
                            ))?
                    };
                    Database::open(&url).await?
                };
                let web_chat_manager = Arc::clone(&chat_manager);
                let web_server = WebServer::from_database_with_configs(
                    &config.web,
                    web_db,
                    &config.files,
                    &config.bbs,
                    config.server.enabled,
                )
                .with_chat_manager(web_chat_manager);
                let web_addr = web_server.addr();

                tokio::spawn(async move {
                    info!("Web server starting on http://{}", web_addr);
                    if let Err(e) = web_server.run().await {
                        error!("Web server error: {}", e);
                    }
                });
            }

            // Start SSH server if enabled (runs in separate task)
            // SSH server is Send-safe (no Cell/RefCell) so tokio::spawn is fine
            if config.ssh.enabled {
                let ssh_config = Arc::new(config.clone());
                tokio::spawn(async move {
                    if let Err(e) = hobbs::server::ssh::run(ssh_config).await {
                        error!("SSH server error: {}", e);
                    }
                });
            }

            // Clone db and config for RSS updater
            let rss_db = Arc::clone(&app.db());
            let rss_config = config.rss.clone();

            // Start RSS background updater (if enabled)
            if start_rss_updater_with_config(rss_db, &rss_config) {
                info!("RSS updater started");
            }

            // Telnet sessions use spawn_local because ScreenContext contains Cell (non-Send)
            loop {
                match server.accept().await {
                    Ok((stream, addr, permit)) => {
                        info!("New connection from {}", addr);
                        let app = app.clone();
                        tokio::task::spawn_local(async move {
                            let mut session = TelnetSession::new(stream, addr);
                            if let Err(e) = app.run_session(&mut session).await {
                                error!("Session error for {}: {}", addr, e);
                            }
                            info!("Connection closed: {}", addr);
                            drop(permit);
                        });
                    }
                    Err(e) => {
                        error!("Accept error: {}", e);
                        break;
                    }
                }
            }

            Ok(())
        })
        .await
}
