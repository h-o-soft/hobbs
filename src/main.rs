use std::sync::Arc;

use tracing::{error, info};

use hobbs::server::SessionManager;
use hobbs::{
    Application, Config, Database, I18nManager, TelnetServer, TelnetSession, TemplateLoader,
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

    // Create application
    let app = Application::new(
        db,
        Arc::new(config.clone()),
        i18n_manager,
        template_loader,
        session_manager,
    );

    // Bind server
    let server = TelnetServer::bind(&config.server).await?;
    info!(
        "Server listening on {}:{}",
        config.server.host, config.server.port
    );
    info!("Press Ctrl+C to stop");

    // Create LocalSet for non-Send futures (rusqlite is not Send)
    let local = tokio::task::LocalSet::new();

    local
        .run_until(async move {
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
