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
    ws.send(Message::Text(serde_json::to_string(&sub).unwrap().into()))
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
    ws.send(Message::Text(serde_json::to_string(&sub).unwrap().into()))
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
    ws.send(Message::Text(serde_json::to_string(&sub).unwrap().into()))
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
async fn net_string_list_metrics_delivered() {
    let url = spawn(make_state()).await;
    let mut ws = connect(&url, Some("secret")).await;
    let _ = ws.next().await.unwrap();

    let sub = SubscribeMessage {
        msg_type: "subscribe".to_string(),
        metrics: vec![
            humours_server::protocol::SubscribeEntry {
                id: "net.interfaces".to_string(),
                refresh_rate_ms: None,
                unit: None,
            },
            humours_server::protocol::SubscribeEntry {
                id: "net.ip_addresses".to_string(),
                refresh_rate_ms: None,
                unit: None,
            },
            humours_server::protocol::SubscribeEntry {
                id: "net.wifi_ssids".to_string(),
                refresh_rate_ms: None,
                unit: None,
            },
            humours_server::protocol::SubscribeEntry {
                id: "net.routes".to_string(),
                refresh_rate_ms: None,
                unit: None,
            },
        ],
    };
    ws.send(Message::Text(serde_json::to_string(&sub).unwrap().into()))
        .await
        .unwrap();

    let mut got_names = false;
    let mut got_ips = false;
    let mut got_ssids = false;
    let mut got_route = false;
    let start = std::time::Instant::now();
    while start.elapsed() < std::time::Duration::from_millis(600) {
        let raw = match tokio::time::timeout(std::time::Duration::from_millis(100), ws.next())
            .await
        {
            Ok(Some(Ok(m))) => m.into_text().unwrap(),
            _ => continue,
        };
        if let Ok(msg) = serde_json::from_str::<DataMessage>(&raw) {
            if let Some(mv) = find_metric(&msg, "net.interfaces") {
                assert!(mv.value.as_string_list().is_some(), "net.interfaces value is not a string list");
                got_names = true;
            }
            if let Some(mv) = find_metric(&msg, "net.ip_addresses") {
                assert!(mv.value.as_string_list().is_some(), "net.ip_addresses value is not a string list");
                got_ips = true;
            }
            if let Some(mv) = find_metric(&msg, "net.wifi_ssids") {
                assert!(mv.value.as_string_list().is_some(), "net.wifi_ssids value is not a string list");
                got_ssids = true;
            }
            if let Some(mv) = find_metric(&msg, "net.routes") {
                let list = mv.value.as_string_list().expect("net.routes value is not a string list");
                assert!(
                    list.iter().all(|e| e.matches(':').count() >= 2),
                    "net.routes entries should be if:route:hop, got {:?}",
                    list
                );
                got_route = true;
            }
        }
        if got_names && got_ips && got_ssids && got_route {
            break;
        }
    }
    assert!(got_names, "never received net.interfaces metric");
    assert!(got_ips, "never received net.ip_addresses static metric");
    assert!(got_ssids, "never received net.wifi_ssids static metric");
    assert!(got_route, "never received net.routes string-list metric");
}

#[tokio::test]
async fn net_rate_and_count_string_list_metrics_delivered() {
    let url = spawn(make_state()).await;
    let mut ws = connect(&url, Some("secret")).await;
    let _ = ws.next().await.unwrap();

    let sub = SubscribeMessage {
        msg_type: "subscribe".to_string(),
        metrics: vec![
            humours_server::protocol::SubscribeEntry {
                id: "net.rx_bytes_rate".to_string(),
                refresh_rate_ms: Some(100),
                unit: None,
            },
            humours_server::protocol::SubscribeEntry {
                id: "net.tx_packets".to_string(),
                refresh_rate_ms: Some(100),
                unit: None,
            },
        ],
    };
    ws.send(Message::Text(serde_json::to_string(&sub).unwrap().into()))
        .await
        .unwrap();

    let mut got_rate = false;
    let mut got_count = false;
    let start = std::time::Instant::now();
    while start.elapsed() < std::time::Duration::from_millis(800) {
        let raw = match tokio::time::timeout(std::time::Duration::from_millis(120), ws.next())
            .await
        {
            Ok(Some(Ok(m))) => m.into_text().unwrap(),
            _ => continue,
        };
        if let Ok(msg) = serde_json::from_str::<DataMessage>(&raw) {
            if let Some(mv) = find_metric(&msg, "net.rx_bytes_rate") {
                let list = mv.value.as_string_list()
                    .expect("net.rx_bytes_rate value is not a string list");
                assert!(list.iter().all(|e| e.contains(':')), "net.rx_bytes_rate entries should be iface:value");
                got_rate = true;
            }
            if let Some(mv) = find_metric(&msg, "net.tx_packets") {
                let list = mv.value.as_string_list()
                    .expect("net.tx_packets value is not a string list");
                assert!(list.iter().all(|e| e.contains(':')), "net.tx_packets entries should be iface:value");
                got_count = true;
            }
        }
        if got_rate && got_count {
            break;
        }
    }
    assert!(got_rate, "never received net.rx_bytes_rate string-list metric");
    assert!(got_count, "never received net.tx_packets count string-list metric");
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
    ws.send(Message::Text(serde_json::to_string(&sub).unwrap().into()))
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

    ws.send(Message::Text("not json at all".to_string().into())).await.unwrap();

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
    ws.send(Message::Text(serde_json::to_string(&sub).unwrap().into()))
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
    ws.send(Message::Text(serde_json::to_string(&sub).unwrap().into()))
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
    ws.send(Message::Text(serde_json::to_string(&sub).unwrap().into()))
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
    ws.send(Message::Text(serde_json::to_string(&sub).unwrap().into()))
        .await
        .unwrap();

    // Collect server-side timestamps of 8 messages. We use the timestamps
    // embedded in the data messages (not wall-clock elapsed time) so the
    // measurement is immune to test-scheduling jitter. At a 50ms rate, 8
    // messages span ~350ms of server time. At 100ms (a rate-floor regression)
    // they would span ~700ms.
    let mut stamps: Vec<u64> = Vec::new();
    let start = std::time::Instant::now();
    while stamps.len() < 8 && start.elapsed() < std::time::Duration::from_secs(3) {
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
    assert!(stamps.len() >= 8, "only got {} data messages", stamps.len());

    let server_span = stamps.last().unwrap() - stamps.first().unwrap();
    assert!(
        server_span < 600,
        "server-side span of 8 messages was {server_span}ms, expected < 600ms for 50ms cadence"
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
