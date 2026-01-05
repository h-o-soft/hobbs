use std::sync::Arc;

use tracing::{error, info};

use hobbs::server::SessionManager;
use hobbs::web::WebServer;
use hobbs::{
    chat::ChatRoomManager, start_rss_updater_with_config, Application, Config, Database,
    I18nManager, TelnetServer, TelnetSession, TemplateLoader,
};

fn main() {
    // Load configuration
    let config = match Config::load("config.toml") {
        Ok(config) => config,
        Err(e) => {
            eprintln!("Failed to load config.toml: {e}");
            eprintln!("Using default configuration.");
            Config::default()
        }
    };

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
    // Ensure data directory exists
    std::fs::create_dir_all("data")?;

    // Open database
    let db = Arc::new(Database::open(&config.database.path)?);
    info!("Database opened: {}", config.database.path);

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
        chat_manager,
    );

    // Bind server
    let server = TelnetServer::bind(&config.server).await?;
    info!(
        "Server listening on {}:{}",
        config.server.host, config.server.port
    );
    info!("Press Ctrl+C to stop");

    // Start Web server if enabled (runs in separate task with its own DB connection)
    if config.web.enabled {
        let web_db = Database::open(&config.database.path)?;
        let web_server = WebServer::from_database_with_files(&config.web, web_db, &config.files);
        let web_addr = web_server.addr();

        tokio::spawn(async move {
            info!("Web server starting on http://{}", web_addr);
            if let Err(e) = web_server.run().await {
                error!("Web server error: {}", e);
            }
        });
    }

    // Create LocalSet for non-Send futures (rusqlite is not Send)
    let local = tokio::task::LocalSet::new();

    // Clone db and config for RSS updater
    let rss_db = Arc::clone(&app.db());
    let rss_config = config.rss.clone();

    local
        .run_until(async move {
            // Start RSS background updater (if enabled)
            if start_rss_updater_with_config(rss_db, &rss_config) {
                info!("RSS updater started");
            }

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
        })
        .await;

    Ok(())
}
