use tracing::info;
use tracing_subscriber;

fn main() {
    tracing_subscriber::fmt::init();
    info!("HOBBS - Hobbyist Bulletin Board System");
}
