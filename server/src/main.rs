use clap::Parser;
use humours_server::config::Cli;
use humours_server::server::AppState;
use std::net::SocketAddr;
use std::sync::Arc;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .init();

    let cli = Cli::parse();
    let config_path = humours_server::config::Config::resolve_or_create(&cli.config)?;
    let config = humours_server::config::Config::load(&config_path)
        .map_err(|e| anyhow::anyhow!("failed to load config from `{}`: {e}", config_path.display()))?;

    let addr: SocketAddr = format!("{}:{}", config.bind_address, config.port)
        .parse()
        .map_err(|e| anyhow::anyhow!("invalid bind address: {e}"))?;

    let catalog = humours_server::hardware::build_catalog();
    let collector = Arc::new(humours_server::hardware::Collector::new());

    let state = AppState {
        config: Arc::new(config.clone()),
        catalog: Arc::new(catalog),
        collector,
    };

    let use_tls = config.tls_cert.is_some() && config.tls_key.is_some();

    if use_tls {
        let cert_path = config.tls_cert.as_ref().unwrap();
        let key_path = config.tls_key.as_ref().unwrap();
        tracing::info!("starting TLS server with provided cert at {addr}");
        serve_tls(state.clone(), addr, cert_path, key_path).await?;
    } else {
        tracing::warn!("no TLS cert configured; generating self-signed certificate on the fly");
        serve_self_signed(state.clone(), addr).await?;
    }

    Ok(())
}

async fn serve_tls(
    state: AppState,
    addr: SocketAddr,
    cert_path: &std::path::Path,
    key_path: &std::path::Path,
) -> anyhow::Result<()> {
    let cert = std::fs::read(cert_path)?;
    let key = std::fs::read(key_path)?;

    let cert_chain: Vec<Vec<u8>> = rustls_pemfile::certs(&mut cert.as_slice())
        .map(|c| c.map(|d| d.to_vec()))
        .collect::<Result<Vec<_>, _>>()?;
    let key: Vec<u8> = rustls_pemfile::pkcs8_private_keys(&mut key.as_slice())
        .next()
        .ok_or_else(|| anyhow::anyhow!("no private key found"))??
        .secret_pkcs8_der()
        .to_vec();

    let config =
        axum_server::tls_rustls::RustlsConfig::from_der(cert_chain, key)
            .await?;

    let app = humours_server::server::router(state);
    axum_server::bind_rustls(addr, config)
        .serve(app.into_make_service())
        .await?;
    Ok(())
}

async fn serve_self_signed(state: AppState, addr: SocketAddr) -> anyhow::Result<()> {
    let cert = rcgen::generate_simple_self_signed(vec!["localhost".to_string(), "127.0.0.1".to_string()])?;
    let cert_der = cert.serialize_der()?;
    let key_der = cert.serialize_private_key_der();

    let config = axum_server::tls_rustls::RustlsConfig::from_der(vec![cert_der], key_der).await?;

    let app = humours_server::server::router(state);
    axum_server::bind_rustls(addr, config)
        .serve(app.into_make_service())
        .await?;
    Ok(())
}
