mod config;
mod hardware;
mod protocol;
mod server;

#[tokio::main]
async fn main() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let cfg = match config::load() {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("failed to load configuration: {}", e);
            return;
        }
    };
    tracing::info!(
        "starting humours server on https://{}:{}",
        cfg.bind_address,
        cfg.port
    );

    if let Err(e) = server::run(cfg).await {
        tracing::error!("server error: {}", e);
    }
}
