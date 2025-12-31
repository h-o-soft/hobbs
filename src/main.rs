use tracing::info;

use hobbs::Config;

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
        "Server configured on {}:{}",
        config.server.host, config.server.port
    );
}
