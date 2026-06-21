use tracing;

mod config;
mod hardware;
mod protocol;
mod server;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let cfg = match config::load() {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("failed to load configuration: {}", e);
            return;
        }
    };
    tracing::info!("starting humours server on https://{}:{}", cfg.bind_address, cfg.port);

    if let Err(e) = server::run(cfg).await {
        tracing::error!("server error: {}", e);
    }
}
