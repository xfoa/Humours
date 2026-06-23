use futures_util::{SinkExt, StreamExt};
use humours_server::config::Config;
use humours_server::hardware::{build_catalog, round_to_quantum, Collector, POLL_QUANTUM_MS};
use humours_server::protocol::{CatalogMessage, DataMessage, MetricDataType, MetricValue, SubscribeMessage};
use humours_server::server::{router, AppState};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio_tungstenite::tungstenite::Message;

fn find_metric<'a>(msg: &'a DataMessage, id: &str) -> Option<&'a MetricValue> {
    msg.metrics.iter().find(|m| m.id == id)
}

fn make_state() -> AppState {
    let config = Config {
        bind_address: "127.0.0.1".to_string(),
        port: 0,
        tls_cert: None,
        tls_key: None,
        auth_token: "secret".to_string(),
        default_refresh_rate_ms: 100,
        poll_interval_ms: 50,
    };
    let catalog = build_catalog();
    let collector = Arc::new(Collector::new());
    AppState {
        config: Arc::new(config),
        catalog: Arc::new(catalog),
        collector,
    }
}

async fn spawn(state: AppState) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let app = router(state);
    tokio::spawn(async move {
        axum::serve(listener, app.into_make_service()).await.unwrap();
    });
    format!("ws://{}", addr)
}

async fn connect(url: &str, token: Option<&str>) -> tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>> {
    let uri = match token {
        Some(t) => format!("{}/ws?token={}", url, t),
        None => format!("{}/ws", url),
    };
    let (ws, _) = tokio_tungstenite::connect_async(uri).await.unwrap();
    ws
}

#[tokio::test]
async fn catalog_sent_on_connect() {
    let url = spawn(make_state()).await;
    let mut ws = connect(&url, Some("secret")).await;

    let raw = ws.next().await.unwrap().unwrap().into_text().unwrap();
    let msg: CatalogMessage = serde_json::from_str(&raw).unwrap();
    assert_eq!(msg.msg_type, "catalog");
    let ids: Vec<_> = msg.metrics.iter().map(|m| m.id.clone()).collect();
    assert!(ids.contains(&"cpu.usage".to_string()));
    assert!(ids.contains(&"mem.used".to_string()));

    let cores = msg.metrics.iter().find(|m| m.id == "cpu.cores").unwrap();
    assert_eq!(cores.data_type, MetricDataType::Integer);
    let cpu_usage = msg.metrics.iter().find(|m| m.id == "cpu.usage").unwrap();
    assert_eq!(cpu_usage.data_type, MetricDataType::Float);
}

#[tokio::test]
async fn bad_token_returns_error() {
    let url = spawn(make_state()).await;
    let mut ws = connect(&url, Some("wrong")).await;

    let raw = ws.next().await.unwrap().unwrap().into_text().unwrap();
    let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(v["type"], "error");
    assert_eq!(v["message"], "unauthorized");
}

#[tokio::test]
async fn subscribe_and_receive_data() {
    let url = spawn(make_state()).await;
    let mut ws = connect(&url, Some("secret")).await;

    // consume catalog
    let _ = ws.next().await.unwrap();

    let sub = SubscribeMessage {
        msg_type: "subscribe".to_string(),
        metrics: vec![
            humours_server::protocol::SubscribeEntry {
                id: "cpu.usage".to_string(),
                refresh_rate_ms: Some(100),
                unit: None,
            },
            humours_server::protocol::SubscribeEntry {
                id: "mem.used".to_string(),
                refresh_rate_ms: Some(100),
                unit: None,
            },
        ],
    };
    ws.send(Message::Text(serde_json::to_string(&sub).unwrap()))
        .await
        .unwrap();

    let mut got_cpu = false;
    let mut got_mem = false;
    for _ in 0..10 {
        let raw = tokio::time::timeout(
            std::time::Duration::from_secs(3),
            ws.next(),
        )
        .await
        .unwrap()
        .unwrap()
        .unwrap()
        .into_text()
        .unwrap();
        let msg: DataMessage = serde_json::from_str(&raw).unwrap();
        assert_eq!(msg.msg_type, "data");
        assert!(msg.timestamp > 0);
        if find_metric(&msg, "cpu.usage").is_some() {
            got_cpu = true;
        }
        if find_metric(&msg, "mem.used").is_some() {
            got_mem = true;
        }
        if got_cpu && got_mem {
            break;
        }
    }
    assert!(got_cpu, "never received cpu.usage");
    assert!(got_mem, "never received mem.used");
}

#[tokio::test]
async fn data_messages_only_contain_subscribed_metrics() {
    let url = spawn(make_state()).await;
    let mut ws = connect(&url, Some("secret")).await;
    let _ = ws.next().await.unwrap();

    let sub = SubscribeMessage {
        msg_type: "subscribe".to_string(),
        metrics: vec![humours_server::protocol::SubscribeEntry {
            id: "cpu.usage".to_string(),
            refresh_rate_ms: Some(100),
            unit: None,
        }],
    };
    ws.send(Message::Text(serde_json::to_string(&sub).unwrap()))
        .await
        .unwrap();

    for _ in 0..5 {
        let raw = tokio::time::timeout(
            std::time::Duration::from_secs(3),
            ws.next(),
        )
        .await
        .unwrap()
        .unwrap()
        .unwrap()
        .into_text()
        .unwrap();
        let msg: DataMessage = serde_json::from_str(&raw).unwrap();
        assert!(
            msg.metrics.len() <= 1,
            "expected only subscribed metrics, got {:?}",
            msg.metrics
        );
    }
}

#[tokio::test]
async fn static_metric_sent_once_on_subscribe() {
    let url = spawn(make_state()).await;
    let mut ws = connect(&url, Some("secret")).await;
    let _ = ws.next().await.unwrap();

    let sub = SubscribeMessage {
        msg_type: "subscribe".to_string(),
        metrics: vec![
            humours_server::protocol::SubscribeEntry {
                id: "cpu.cores".to_string(),
                refresh_rate_ms: None,
                unit: None,
            },
            humours_server::protocol::SubscribeEntry {
                id: "cpu.usage".to_string(),
                refresh_rate_ms: Some(50),
                unit: None,
            },
        ],
    };
    ws.send(Message::Text(serde_json::to_string(&sub).unwrap()))
        .await
        .unwrap();

    let mut cores_seen = 0;
    let start = std::time::Instant::now();
    while start.elapsed() < std::time::Duration::from_millis(400) {
        let raw = match tokio::time::timeout(std::time::Duration::from_millis(100), ws.next())
            .await
        {
            Ok(Some(Ok(m))) => m.into_text().unwrap(),
            _ => continue,
        };
        if let Ok(msg) = serde_json::from_str::<DataMessage>(&raw) {
            if find_metric(&msg, "cpu.cores").is_some() {
                cores_seen += 1;
            }
        }
    }
    assert_eq!(
        cores_seen, 1,
        "cpu.cores (static) was sent {cores_seen} times, expected 1"
    );
}

#[tokio::test]
async fn refresh_rate_on_static_metric_returns_error() {
    let url = spawn(make_state()).await;
    let mut ws = connect(&url, Some("secret")).await;
    let _ = ws.next().await.unwrap();

    let sub = SubscribeMessage {
        msg_type: "subscribe".to_string(),
        metrics: vec![humours_server::protocol::SubscribeEntry {
            id: "cpu.cores".to_string(),
            refresh_rate_ms: Some(100),
            unit: None,
        }],
    };
    ws.send(Message::Text(serde_json::to_string(&sub).unwrap()))
        .await
        .unwrap();

    let mut got_error = false;
    let start = std::time::Instant::now();
    while start.elapsed() < std::time::Duration::from_millis(300) {
        let raw = match tokio::time::timeout(std::time::Duration::from_millis(100), ws.next())
            .await
        {
            Ok(Some(Ok(m))) => m.into_text().unwrap(),
            _ => continue,
        };
        let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
        if v["type"] == "error" {
            assert!(
                v["message"].as_str().unwrap().contains("static"),
                "unexpected error message: {v}"
            );
            got_error = true;
            break;
        }
    }
    assert!(got_error, "expected an error for refresh_rate on static metric");
}

#[tokio::test]
async fn malformed_json_returns_error() {
    let url = spawn(make_state()).await;
    let mut ws = connect(&url, Some("secret")).await;
    let _ = ws.next().await.unwrap();

    ws.send(Message::Text("not json at all".to_string())).await.unwrap();

    let mut got_error = false;
    let start = std::time::Instant::now();
    while start.elapsed() < std::time::Duration::from_millis(300) {
        let raw = match tokio::time::timeout(std::time::Duration::from_millis(100), ws.next())
            .await
        {
            Ok(Some(Ok(m))) => m.into_text().unwrap(),
            _ => continue,
        };
        let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
        if v["type"] == "error" {
            assert!(
                v["message"].as_str().unwrap().contains("invalid subscribe message"),
                "unexpected error: {v}"
            );
            got_error = true;
            break;
        }
    }
    assert!(got_error, "expected error for malformed JSON");
}

#[tokio::test]
async fn custom_unit_is_honored() {
    let url = spawn(make_state()).await;
    let mut ws = connect(&url, Some("secret")).await;
    let _ = ws.next().await.unwrap();

    let sub = SubscribeMessage {
        msg_type: "subscribe".to_string(),
        metrics: vec![humours_server::protocol::SubscribeEntry {
            id: "mem.total".to_string(),
            refresh_rate_ms: None,
            unit: Some("MB".to_string()),
        }],
    };
    ws.send(Message::Text(serde_json::to_string(&sub).unwrap()))
        .await
        .unwrap();

    let mut got_value = false;
    let start = std::time::Instant::now();
    while start.elapsed() < std::time::Duration::from_millis(300) {
        let raw = match tokio::time::timeout(std::time::Duration::from_millis(100), ws.next())
            .await
        {
            Ok(Some(Ok(m))) => m.into_text().unwrap(),
            _ => continue,
        };
        if let Ok(msg) = serde_json::from_str::<DataMessage>(&raw) {
            if let Some(mv) = find_metric(&msg, "mem.total") {
                // mem.total in MB should be in the hundreds-to-thousands range,
                // not single-digit GB.
                assert!(mv.value.as_f64() > 100.0, "mem.total in MB was {}, expected > 100", mv.value.as_f64());
                got_value = true;
                break;
            }
        }
    }
    assert!(got_value, "never received mem.total with custom unit");
}

#[tokio::test]
async fn data_message_includes_units() {
    let url = spawn(make_state()).await;
    let mut ws = connect(&url, Some("secret")).await;
    let _ = ws.next().await.unwrap();

    let sub = SubscribeMessage {
        msg_type: "subscribe".to_string(),
        metrics: vec![
            humours_server::protocol::SubscribeEntry {
                id: "mem.total".to_string(),
                refresh_rate_ms: None,
                unit: Some("MB".to_string()),
            },
            humours_server::protocol::SubscribeEntry {
                id: "cpu.usage".to_string(),
                refresh_rate_ms: Some(100),
                unit: None,
            },
        ],
    };
    ws.send(Message::Text(serde_json::to_string(&sub).unwrap()))
        .await
        .unwrap();

    let mut checked_mem = false;
    let mut checked_cpu = false;
    let start = std::time::Instant::now();
    while start.elapsed() < std::time::Duration::from_millis(500) && !(checked_mem && checked_cpu) {
        let raw = match tokio::time::timeout(std::time::Duration::from_millis(100), ws.next())
            .await
        {
            Ok(Some(Ok(m))) => m.into_text().unwrap(),
            _ => continue,
        };
        if let Ok(msg) = serde_json::from_str::<DataMessage>(&raw) {
            if let Some(mv) = find_metric(&msg, "mem.total") {
                assert_eq!(mv.unit, "MB");
                assert!(mv.value.as_f64() > 100.0);
                checked_mem = true;
            }
            if let Some(mv) = find_metric(&msg, "cpu.usage") {
                assert_eq!(mv.unit, "%");
                checked_cpu = true;
            }
        }
    }
    assert!(checked_mem, "never received mem.total");
    assert!(checked_cpu, "never received cpu.usage");
}

#[tokio::test]
async fn invalid_unit_returns_error() {
    let url = spawn(make_state()).await;
    let mut ws = connect(&url, Some("secret")).await;
    let _ = ws.next().await.unwrap();

    let sub = SubscribeMessage {
        msg_type: "subscribe".to_string(),
        metrics: vec![humours_server::protocol::SubscribeEntry {
            id: "mem.total".to_string(),
            refresh_rate_ms: None,
            unit: Some("frobnicate".to_string()),
        }],
    };
    ws.send(Message::Text(serde_json::to_string(&sub).unwrap()))
        .await
        .unwrap();

    let mut got_error = false;
    let start = std::time::Instant::now();
    while start.elapsed() < std::time::Duration::from_millis(300) {
        let raw = match tokio::time::timeout(std::time::Duration::from_millis(100), ws.next())
            .await
        {
            Ok(Some(Ok(m))) => m.into_text().unwrap(),
            _ => continue,
        };
        let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
        if v["type"] == "error" {
            assert!(
                v["message"].as_str().unwrap().contains("not valid"),
                "unexpected error: {v}"
            );
            got_error = true;
            break;
        }
    }
    assert!(got_error, "expected error for invalid unit");
}

#[tokio::test]
async fn sub_50ms_rate_fires_within_quantum_window() {
    let url = spawn(make_state()).await;
    let mut ws = connect(&url, Some("secret")).await;
    let _ = ws.next().await.unwrap();

    // 50ms is the minimum; request 1ms which must round up to 50ms.
    let sub = SubscribeMessage {
        msg_type: "subscribe".to_string(),
        metrics: vec![humours_server::protocol::SubscribeEntry {
            id: "cpu.usage".to_string(),
            refresh_rate_ms: Some(1),
            unit: None,
        }],
    };
    ws.send(Message::Text(serde_json::to_string(&sub).unwrap()))
        .await
        .unwrap();

    // Collect timestamps of 5 messages and confirm they arrive at ~50ms cadence,
    // not ~100ms. Allow generous slack for CI scheduling.
    let mut stamps: Vec<u64> = Vec::new();
    let start = std::time::Instant::now();
    while stamps.len() < 5 && start.elapsed() < std::time::Duration::from_secs(3) {
        let raw = tokio::time::timeout(std::time::Duration::from_millis(200), ws.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap()
            .into_text()
            .unwrap();
        let msg: DataMessage = serde_json::from_str(&raw).unwrap();
        stamps.push(msg.timestamp);
    }
    assert!(stamps.len() >= 5, "only got {} data messages", stamps.len());

    // With a 50ms rate we expect ~5 messages within ~300ms. If we were stuck
    // at 100ms we'd see gaps >= ~100ms. Check that the median inter-arrival
    // gap is closer to 50ms than 100ms.
    let mut gaps: Vec<u64> = stamps.windows(2).map(|w| w[1].saturating_sub(w[0])).collect();
    gaps.sort();
    let median = gaps[gaps.len() / 2];
    assert!(
        median <= 70,
        "expected ~50ms cadence, median inter-arrival gap was {median}ms"
    );
}

#[test]
fn round_to_quantum_aligns_to_50ms() {
    assert_eq!(round_to_quantum(0), POLL_QUANTUM_MS);
    assert_eq!(round_to_quantum(50), 50);
    assert_eq!(round_to_quantum(51), 100);
    assert_eq!(round_to_quantum(99), 100);
    assert_eq!(round_to_quantum(100), 100);
    assert_eq!(round_to_quantum(150), 150);
    assert_eq!(round_to_quantum(1234), 1250);
}
