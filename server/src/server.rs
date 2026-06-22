use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use futures_util::{SinkExt, StreamExt};
use axum::extract::{Query, State};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use tokio::sync::{broadcast, RwLock};
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
    tracing::debug!("catalog has {} metrics", catalog.len());
    let (tx, _) = broadcast::channel(1024);
    tracing::debug!("broadcast channel created with capacity 1024");

    let state = Arc::new(AppState {
        catalog: catalog.clone(),
        tx: tx.clone(),
        auth_token: cfg.auth_token.clone(),
        subscriptions: Arc::new(RwLock::new(HashMap::new())),
    });
    tracing::debug!("app state initialized, auth_token length = {}", state.auth_token.len());

    // Start the metric collection loop
    let state_clone = state.clone();
    tokio::spawn(metric_collection_loop(state_clone));

    let app = Router::new()
        .route("/ws", get(ws_handler))
        .with_state(state);

    let addr = format!("{}:{}", cfg.bind_address, cfg.port).parse::<std::net::SocketAddr>()?;

    tracing::info!("starting humours server on ws://{}:{}", cfg.bind_address, cfg.port);

    axum_server::bind(addr)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}


async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<HashMap<String, String>>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let token = params.get("token").cloned().unwrap_or_default();
    tracing::debug!("websocket connection attempt, token present = {}", !token.is_empty());
    tracing::info!("websocket connection attempt, token={}", token);
    if token != state.auth_token {
        tracing::warn!("unauthorized connection attempt with token={}", token);
        return (axum::http::StatusCode::UNAUTHORIZED, "invalid token").into_response();
    }
    tracing::debug!("token accepted, upgrading to websocket");
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: Arc<AppState>) {
    tracing::debug!("handle_socket started");
    tracing::info!("websocket connected, sending catalog");
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

    let catalog_text = format!("{}\n", catalog_text);
    tracing::debug!("catalog message: {}", catalog_text);
    if let Err(e) = socket.send(Message::Text(catalog_text.into())).await {
        tracing::error!("failed to send catalog: {}", e);
        return;
    }
    tracing::info!("catalog sent successfully");

    let mut rx = state.tx.subscribe();
    let client_subscriptions: Arc<RwLock<Vec<MetricSubscription>>> = Arc::new(RwLock::new(Vec::new()));

    let (mut sink, mut stream) = socket.split();

    let recv_task = {
        let client_subscriptions = client_subscriptions.clone();
        let state = state.clone();
        async move {
            while let Some(Ok(msg)) = stream.next().await {
                match msg {
                    Message::Text(text) => {
                        tracing::info!("received message: {}", text);
                        tracing::debug!("parsing client message: {}", text);
                        match serde_json::from_str::<ClientMessage>(&text) {
                            Ok(ClientMessage::Subscribe { metrics }) => {
                                let mut subs = client_subscriptions.write().await;
                                *subs = metrics.clone();
                                let mut global_subs = state.subscriptions.write().await;
                                for sub in &metrics {
                                    let quantized = quantize_interval(sub.refresh_rate_ms, 50);
                                    global_subs.insert(sub.id.clone(), quantized);
                                }
                                tracing::info!("client subscribed to {} metrics: {:?}", metrics.len(), metrics);
                                tracing::debug!("updated global subscriptions: {:?}", global_subs.clone());
                            }
                            Err(e) => {
                                tracing::warn!("invalid client message: {}", e);
                                tracing::debug!("failed to parse: {}", text);
                            }
                        }
                    }
                    Message::Close(_) => {
                        tracing::debug!("received close frame, exiting recv_task");
                        break;
                    }
                    other => {
                        tracing::debug!("received non-text websocket message: {:?}", other);
                    }
                }
            }
            tracing::debug!("recv_task ended");
        }
    };

    let send_task = {
        let client_subscriptions = client_subscriptions.clone();
        async move {
            loop {
                let msg = match rx.recv().await {
                    Ok(msg) => msg,
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!("broadcast lagged by {} messages, skipping", n);
                        continue;
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        tracing::debug!("broadcast channel closed, exiting send_task");
                        break;
                    }
                };
                let subscribed_ids: std::collections::HashSet<String> = client_subscriptions
                    .read()
                    .await
                    .iter()
                    .map(|s| s.id.clone())
                    .collect();
                tracing::debug!("send_task got msg, client subscriptions = {:?}", subscribed_ids);

                let filtered_msg = match &msg {
                    ServerMessage::Data { timestamp, values } => {
                        let filtered: HashMap<String, f64> = values
                            .iter()
                            .filter(|(k, _)| subscribed_ids.contains(*k))
                            .map(|(k, v)| (k.clone(), *v))
                            .collect();
                        tracing::debug!("filtered values from {} to {} keys", values.len(), filtered.len());
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
                let text = format!("{}\n", text);
                tracing::debug!("sending to websocket: {}", text);
                if sink.send(Message::Text(text.into())).await.is_err() {
                    tracing::debug!("websocket send failed, closing send_task");
                    break;
                }
                if sink.flush().await.is_err() {
                    tracing::debug!("websocket flush failed, closing send_task");
                    break;
                }
            }
            tracing::debug!("send_task ended");
        }
    };

    tokio::select! {
        _ = recv_task => {}
        _ = send_task => {}
    }

    let subs = client_subscriptions.read().await.clone();
    let mut global_subs = state.subscriptions.write().await;
    for sub in &subs {
        global_subs.remove(&sub.id);
    }
    tracing::debug!("removed client subscriptions, global_subs = {:?}", global_subs.clone());
    tracing::info!("websocket disconnected");
}

async fn metric_collection_loop(state: Arc<AppState>) {
    let mut collector = MetricCollector::new();
    let mut tick: u64 = 0;
    let grid_ms: u64 = 50;
    let mut interval = tokio::time::interval(Duration::from_millis(grid_ms));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    tracing::debug!("metric collection loop started with {} ms grid", grid_ms);

    loop {
        interval.tick().await;
        tick = tick.wrapping_add(1);

        let receiver_count = state.tx.receiver_count();
        if receiver_count == 0 {
            let mut subs = state.subscriptions.write().await;
            subs.clear();
            tracing::debug!("no receivers, clearing subscriptions");
            continue;
        }

        let subscriptions = state.subscriptions.read().await.clone();
        tracing::debug!("metric loop tick {}, subscriptions = {:?}", tick, subscriptions);
        if subscriptions.is_empty() {
            continue;
        }

        collector.begin_batch();
        let mut values = HashMap::new();
        for (metric_id, refresh_rate_ms) in &subscriptions {
            let quantized = quantize_interval(*refresh_rate_ms, grid_ms);
            if tick % (quantized / grid_ms) == 0 {
                if let Some(val) = collector.get_value(metric_id) {
                    tracing::info!("collected metric {} = {}", metric_id, val);
                    values.insert(metric_id.clone(), val);
                } else {
                    tracing::warn!("metric {} not available", metric_id);
                    tracing::debug!("collector returned None for metric_id: {}", metric_id);
                }
            }
        }

        if !values.is_empty() {
            let msg = ServerMessage::Data {
                timestamp: chrono::Utc::now().timestamp_millis(),
                values: values.clone(),
            };
            tracing::info!("broadcasting data to {} metrics", values.len());
            tracing::debug!("broadcast values: {:?}", values);
            let _ = state.tx.send(msg);
        }
    }
}

fn quantize_interval(value: u64, step: u64) -> u64 {
    if value == 0 {
        return step;
    }
    ((value + step - 1) / step) * step
}
