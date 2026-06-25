use crate::config::Config;
use crate::hardware::{round_to_quantum, Collector};
use crate::protocol::{
    CatalogMessage, DataMessage, ErrorMessage, MetricDataType, SubscribeMessage,
};
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query, State,
    },
    response::IntoResponse,
    routing::get,
    Router,
};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, watch, Notify};
use tokio::time::interval;
use tracing::{debug, error, info, warn};

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub catalog: Arc<Vec<crate::protocol::CatalogMetric>>,
    pub catalog_index: Arc<HashMap<String, crate::protocol::CatalogMetric>>,
    pub collector: Arc<Collector>,
}

impl AppState {
    pub fn new(config: Arc<Config>, catalog: Vec<crate::protocol::CatalogMetric>, collector: Arc<Collector>) -> Self {
        let catalog_index = catalog.iter().map(|m| (m.id.clone(), m.clone())).collect();
        AppState {
            config,
            catalog: Arc::new(catalog),
            catalog_index: Arc::new(catalog_index),
            collector,
        }
    }

    pub fn catalog_entry(&self, id: &str) -> Option<&crate::protocol::CatalogMetric> {
        self.catalog_index.get(id)
    }
}

#[derive(Clone, Debug)]
struct Subscription {
    rate: u64,
    unit: String,
    data_type: MetricDataType,
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[derive(Clone)]
enum Outgoing {
    Data(Arc<String>),
    Error(Arc<String>),
}

impl Outgoing {
    fn text(&self) -> &str {
        match self {
            Outgoing::Data(s) => s.as_str(),
            Outgoing::Error(s) => s.as_str(),
        }
    }
}

fn data_msg(metrics: Vec<crate::protocol::MetricValue>) -> Outgoing {
    let msg = DataMessage::new(now_ms(), metrics);
    let text = serde_json::to_string(&msg).unwrap_or_else(|e| {
        error!("failed to serialize data message: {e}");
        String::new()
    });
    Outgoing::Data(Arc::new(text))
}

fn error_msg<S: Into<String>>(message: S) -> Outgoing {
    let msg = ErrorMessage::new(message);
    let text = serde_json::to_string(&msg).unwrap_or_else(|e| {
        error!("failed to serialize error message: {e}");
        String::new()
    });
    Outgoing::Error(Arc::new(text))
}

#[derive(Deserialize)]
pub struct AuthQuery {
    pub token: Option<String>,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/ws", get(ws_handler))
        .route("/health", get(health_handler))
        .with_state(state)
}

async fn health_handler() -> impl IntoResponse {
    "ok"
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(auth): Query<AuthQuery>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let auth_disabled = state.config.auth_token.is_empty();
    if !auth_disabled && Some(state.config.auth_token.as_str()) != auth.token.as_deref() {
        return ws.on_upgrade(|mut socket| async move {
            let _ = socket
                .send(Message::Text(
                    serde_json::to_string(&ErrorMessage::new("unauthorized"))
                        .unwrap()
                        .into(),
                ))
                .await;
            let _ = socket.close().await;
        });
    }
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: AppState) {
    let (mut sink, mut stream) = socket.split();

    let catalog_msg =
        serde_json::to_string(&CatalogMessage::new(state.catalog.as_ref().clone()));
    match catalog_msg {
        Ok(text) => {
            if sink.send(Message::Text(text.into())).await.is_err() {
                return;
            }
        }
        Err(e) => {
            error!("failed to serialize catalog: {e}");
            return;
        }
    }

    let buffer = state.config.broadcast_buffer.max(64);
    let (out_tx, _rx0) = broadcast::channel::<Outgoing>(buffer);
    let (sub_tx, sub_rx) = watch::channel(HashMap::<String, Subscription>::new());
    let shutdown = Arc::new(Notify::new());

    let poll_handle = tokio::spawn(poll_loop(
        state.clone(),
        out_tx.clone(),
        sub_rx,
        shutdown.clone(),
    ));
    let send_out_tx = out_tx.clone();
    let send_shutdown = shutdown.clone();

    let send_handle = tokio::spawn(async move {
        let mut rx = send_out_tx.subscribe();
        loop {
            tokio::select! {
                biased;
                _ = send_shutdown.notified() => break,
                msg = rx.recv() => {
                    match msg {
                        Ok(outgoing) => {
                            if sink.send(Message::Text(outgoing.text().to_string().into())).await.is_err() {
                                break;
                            }
                        }
                        Err(broadcast::error::RecvError::Lagged(n)) => {
                            debug!("client lagged, dropped {n} messages");
                            continue;
                        }
                        Err(broadcast::error::RecvError::Closed) => break,
                    }
                }
            }
        }
    });

    let recv_handle = tokio::spawn(async move {
        while let Some(Ok(msg)) = stream.next().await {
            match msg {
                Message::Text(text) => {
                    let parsed: Result<SubscribeMessage, _> = serde_json::from_str(&text);
                    match parsed {
                        Ok(sub) => {
                            debug!("subscribe: {:?}", sub.metrics);
                            match process_subscription(&state, &sub) {
                                SubscriptionResult::Ok { map, static_reqs } => {
                                    if !static_reqs.is_empty() {
                                        let metrics = state.collector.sample_many(&static_reqs);
                                        if !metrics.is_empty() {
                                            let _ = out_tx.send(data_msg(metrics));
                                        }
                                    }
                                    if sub_tx.send(map).is_err() {
                                        break;
                                    }
                                }
                                SubscriptionResult::Errors(errors) => {
                                    for err in errors {
                                        let _ = out_tx.send(error_msg(err));
                                    }
                                    let _ = out_tx.send(error_msg(
                                        "subscribe rejected due to previous error(s)",
                                    ));
                                }
                            }
                        }
                        Err(e) => {
                            warn!("bad subscribe message: {e}");
                            let _ = out_tx.send(error_msg(format!("invalid subscribe message: {e}")));
                        }
                    }
                }
                Message::Close(_) => break,
                _ => {}
            }
        }
    });

    let _ = recv_handle.await;
    shutdown.notify_waiters();
    let _ = send_handle.await;
    let _ = poll_handle.await;
    info!("client disconnected");
}

enum SubscriptionResult {
    Ok {
        map: HashMap<String, Subscription>,
        static_reqs: Vec<(String, String, MetricDataType)>,
    },
    Errors(Vec<String>),
}

fn process_subscription(
    state: &AppState,
    sub: &SubscribeMessage,
) -> SubscriptionResult {
    let mut map = HashMap::new();
    let mut static_reqs: Vec<(String, String, MetricDataType)> = Vec::new();
    let mut errors: Vec<String> = Vec::new();

    for entry in &sub.metrics {
        let metric = match state.catalog_entry(&entry.id) {
            Some(m) => m,
            None => {
                errors.push(format!("unknown metric `{}`", entry.id));
                continue;
            }
        };
        let unit = match &entry.unit {
            Some(u) => {
                if !metric.available_units.iter().any(|a| a == u) {
                    errors.push(format!("unit `{}` is not valid for metric `{}`", u, entry.id));
                    continue;
                }
                u.clone()
            }
            None => metric.default_unit.clone(),
        };
        if metric.r#static {
            if entry.refresh_rate_ms.is_some() {
                errors.push(format!(
                    "metric `{}` is static; refresh_rate_ms is not allowed",
                    entry.id
                ));
                continue;
            }
            static_reqs.push((entry.id.clone(), unit, metric.data_type));
        } else {
            let rate = entry
                .refresh_rate_ms
                .map(round_to_quantum)
                .unwrap_or_else(|| round_to_quantum(state.config.default_refresh_rate_ms));
            map.insert(entry.id.clone(), Subscription { rate, unit, data_type: metric.data_type });
        }
    }

    if errors.is_empty() {
        SubscriptionResult::Ok { map, static_reqs }
    } else {
        SubscriptionResult::Errors(errors)
    }
}

async fn poll_loop(
    state: AppState,
    tx: broadcast::Sender<Outgoing>,
    mut sub_rx: watch::Receiver<HashMap<String, Subscription>>,
    shutdown: Arc<Notify>,
) {
    let quantum = state.config.poll_interval_ms.max(50);
    let mut tick = interval(Duration::from_millis(quantum));
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    let mut current_subs: HashMap<String, Subscription> = HashMap::new();
    let mut tick_count: u64 = 0;
    let mut force_sample = false;

    loop {
        tokio::select! {
            biased;
            _ = shutdown.notified() => return,
            _ = tick.tick() => {}
            Ok(()) = sub_rx.changed() => {
                current_subs = sub_rx.borrow_and_update().clone();
                debug!("subscriptions updated: {:?}", current_subs);
                force_sample = true;
                continue;
            }
        }

        if current_subs.is_empty() {
            tick_count = tick_count.wrapping_add(1);
            continue;
        }

        let mut due: Vec<(String, String, MetricDataType)> = Vec::new();
        for (id, sub) in current_subs.iter() {
            let ticks_per_sample = (sub.rate / quantum).max(1);
            if force_sample || tick_count.is_multiple_of(ticks_per_sample) {
                due.push((id.clone(), sub.unit.clone(), sub.data_type));
            }
        }
        force_sample = false;
        tick_count = tick_count.wrapping_add(1);

        if due.is_empty() {
            continue;
        }

        let metrics = state.collector.sample_many(&due);

        if metrics.is_empty() {
            continue;
        }

        let _ = tx.send(data_msg(metrics));
    }
}
