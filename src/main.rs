use tracing::info;

fn main() {
    tracing_subscriber::fmt::init();
    info!("HOBBS - Hobbyist Bulletin Board System");
}
