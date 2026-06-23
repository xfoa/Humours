use axum_server::Handle;
use clap::Parser;
use humours_server::config::Cli;
use humours_server::server::AppState;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
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

    let tls_config = match (&config.tls_cert, &config.tls_key) {
        (Some(cert_path), Some(key_path)) => {
            tracing::info!("starting TLS server with provided cert at {addr}");
            Some(load_tls_config(cert_path, key_path).await?)
        }
        (Some(_), None) | (None, Some(_)) => {
            anyhow::bail!("tls_cert and tls_key must both be set or both be omitted");
        }
        (None, None) => {
            tracing::warn!("no TLS cert configured; generating self-signed certificate on the fly");
            Some(self_signed_tls_config().await?)
        }
    };

    let app = humours_server::server::router(state);
    let handle = Handle::new();

    tokio::spawn({
        let handle = handle.clone();
        async move {
            tokio::signal::ctrl_c().await.ok();
            tracing::info!("shutdown signal received");
            handle.graceful_shutdown(Some(Duration::from_secs(5)));
        }
    });

    if let Some(tls) = tls_config {
        axum_server::bind_rustls(addr, tls)
            .handle(handle)
            .serve(app.into_make_service())
            .await?;
    } else {
        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, app.into_make_service())
            .with_graceful_shutdown(async {
                tokio::signal::ctrl_c().await.ok();
            })
            .await?;
    }

    Ok(())
}

async fn load_tls_config(
    cert_path: &std::path::Path,
    key_path: &std::path::Path,
) -> anyhow::Result<axum_server::tls_rustls::RustlsConfig> {
    let cert = std::fs::read(cert_path)?;
    let key = std::fs::read(key_path)?;

    let cert_chain: Vec<Vec<u8>> = rustls_pemfile::certs(&mut cert.as_slice())
        .map(|c| c.map(|d| d.to_vec()))
        .collect::<Result<Vec<_>, _>>()?;
    if cert_chain.is_empty() {
        anyhow::bail!("no certificates found in {}", cert_path.display());
    }

    let key_vec = read_private_key(&key)?;

    Ok(axum_server::tls_rustls::RustlsConfig::from_der(cert_chain, key_vec).await?)
}

fn read_private_key(key_pem: &[u8]) -> anyhow::Result<Vec<u8>> {
    let mut reader = std::io::BufReader::new(key_pem);
    if let Some(item) = rustls_pemfile::pkcs8_private_keys(&mut reader).next() {
        return Ok(item?.secret_pkcs8_der().to_vec());
    }
    let mut reader = std::io::BufReader::new(key_pem);
    if let Some(item) = rustls_pemfile::rsa_private_keys(&mut reader).next() {
        return Ok(item?.secret_pkcs1_der().to_vec());
    }
    let mut reader = std::io::BufReader::new(key_pem);
    if let Some(item) = rustls_pemfile::ec_private_keys(&mut reader).next() {
        return Ok(item?.secret_sec1_der().to_vec());
    }
    anyhow::bail!("no private key found (tried PKCS#8, RSA, and EC)");
}

async fn self_signed_tls_config() -> anyhow::Result<axum_server::tls_rustls::RustlsConfig> {
    let cert = rcgen::generate_simple_self_signed(vec!["localhost".to_string(), "127.0.0.1".to_string()])?;
    let cert_der = cert.serialize_der()?;
    let key_der = cert.serialize_private_key_der();
    Ok(axum_server::tls_rustls::RustlsConfig::from_der(vec![cert_der], key_der).await?)
}
