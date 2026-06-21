use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Query, State};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use axum_server::tls_rustls::RustlsConfig;
use tokio::sync::{broadcast, RwLock};
use tokio::time::interval;
use tracing;

use crate::config::Config;
use crate::hardware::{discover_metrics, MetricCollector, MetricCatalog};
use crate::protocol::{ClientMessage, MetricSubscription, ServerMessage};

#[derive(Clone)]
pub struct AppState {
    pub catalog: Vec<MetricCatalog>,
    pub tx: broadcast::Sender<ServerMessage>,
    pub auth_token: String,
    pub subscriptions: Arc<RwLock<HashMap<String, u64>>>,
}

pub async fn run(cfg: Config) -> anyhow::Result<()> {
    let catalog = discover_metrics();
    let (tx, _rx) = broadcast::channel(128);

    let state = Arc::new(AppState {
        catalog: catalog.clone(),
        tx: tx.clone(),
        auth_token: cfg.auth_token.clone(),
        subscriptions: Arc::new(RwLock::new(HashMap::new())),
    });

    // Ensure TLS certificates exist
    ensure_certs(&cfg.tls_cert, &cfg.tls_key).await?;

    // Start the metric collection loop
    let state_clone = state.clone();
    tokio::spawn(metric_collection_loop(state_clone));

    let app = Router::new()
        .route("/ws", get(ws_handler))
        .with_state(state);

    let addr = format!("{}:{}", cfg.bind_address, cfg.port).parse()?;
    let tls_config = RustlsConfig::from_pem_file(&cfg.tls_cert, &cfg.tls_key).await?;

    tracing::info!("starting humours server on https://{}:{}", cfg.bind_address, cfg.port);

    axum_server::bind_rustls(addr, tls_config)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}

async fn ensure_certs(cert_path: &str, key_path: &str) -> anyhow::Result<()> {
    if std::path::Path::new(cert_path).exists() && std::path::Path::new(key_path).exists() {
        return Ok(());
    }

    tracing::info!("generating self-signed TLS certificates");
    let cert = rcgen::generate_simple_self_signed(vec!["localhost".into()])?;
    let cert_pem = cert.cert.pem();
    let key_pem = cert.key_pair.serialize_pem();

    tokio::fs::write(cert_path, cert_pem).await?;
    tokio::fs::write(key_path, key_pem).await?;
    Ok(())
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<HashMap<String, String>>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let token = params.get("token").cloned().unwrap_or_default();
    if token != state.auth_token {
        return (axum::http::StatusCode::UNAUTHORIZED, "invalid token").into_response();
    }
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: Arc<AppState>) {
    // Send catalog immediately upon connection
    let catalog_msg = ServerMessage::Catalog {
        metrics: state.catalog.clone(),
    };
    let catalog_text = match serde_json::to_string(&catalog_msg) {
        Ok(t) => t,
        Err(e) => {
            tracing::error!("failed to serialize catalog: {}", e);
            return;
        }
    };
    if let Err(e) = socket.send(Message::Text(catalog_text)).await {
        tracing::error!("failed to send catalog: {}", e);
        return;
    }

    let mut rx = state.tx.subscribe();
    let mut client_subscriptions: Vec<MetricSubscription> = Vec::new();

    let recv_task = {
        let state = state.clone();
        async move {
            while let Some(Ok(msg)) = socket.recv().await {
                if let Message::Text(text) = msg {
                    match serde_json::from_str::<ClientMessage>(&text) {
                        Ok(ClientMessage::Subscribe { metrics }) => {
                            client_subscriptions = metrics.clone();
                            let mut subs = state.subscriptions.write().await;
                            for sub in &metrics {
                                let rounded = ((sub.refresh_rate_ms + 49) / 50) * 50;
                                subs.insert(sub.id.clone(), rounded.max(50));
                            }
                            tracing::debug!("client subscribed to {:?} metrics", metrics.len());
                        }
                        Err(e) => {
                            tracing::warn!("invalid client message: {}", e);
                        }
                    }
                }
            }
        }
    };

    let send_task = async move {
        let subscribed_ids: Vec<String> = client_subscriptions.iter().map(|s| s.id.clone()).collect();
        while let Ok(msg) = rx.recv().await {
            let filtered_msg = match &msg {
                ServerMessage::Data { timestamp, values } => {
                    let filtered: HashMap<String, f64> = values
                        .iter()
                        .filter(|(k, _)| subscribed_ids.contains(k))
                        .map(|(k, v)| (k.clone(), *v))
                        .collect();
                    if filtered.is_empty() {
                        continue;
                    }
                    ServerMessage::Data {
                        timestamp: *timestamp,
                        values: filtered,
                    }
                }
                _ => msg,
            };

            let text = match serde_json::to_string(&filtered_msg) {
                Ok(t) => t,
                Err(e) => {
                    tracing::error!("failed to serialize data: {}", e);
                    continue;
                }
            };
            if socket.send(Message::Text(text)).await.is_err() {
                break;
            }
        }
    };

    tokio::select! {
        _ = recv_task => {}
        _ = send_task => {}
    }

    tracing::info!("websocket disconnected");
}

async fn metric_collection_loop(state: Arc<AppState>) {
    let mut collector = MetricCollector::new();
    let mut tick = interval(Duration::from_millis(50));
    let mut last_update: HashMap<String, std::time::Instant> = HashMap::new();

    loop {
        tick.tick().await;

        let subscriptions = state.subscriptions.read().await.clone();
        if subscriptions.is_empty() {
            continue;
        }

        let mut values = HashMap::new();
        let now = std::time::Instant::now();

        for (metric_id, refresh_rate_ms) in &subscriptions {
            let due = match last_update.get(metric_id) {
                Some(last) => now.duration_since(*last).as_millis() >= *refresh_rate_ms as u128,
                None => true,
            };

            if due {
                if let Some(val) = collector.get_value(metric_id) {
                    values.insert(metric_id.clone(), val);
                }
                last_update.insert(metric_id.clone(), now);
            }
        }

        if !values.is_empty() {
            let msg = ServerMessage::Data {
                timestamp: chrono::Utc::now().timestamp_millis(),
                values,
            };
            let _ = state.tx.send(msg);
        }
    }
}
