use humours_server::protocol::{ClientMessage, MetricSubscription};
use std::time::Instant;

use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::{connect_async, tungstenite::Message};

#[tokio::test]
async fn fast_subscribe_client() {
    let url = "ws://localhost:8443/ws?token=dev-token";
    let (mut ws, _) = connect_async(url).await.expect("connect");

    let msg = ClientMessage::Subscribe {
        metrics: vec![MetricSubscription {
            id: "cpu.usage".into(),
            refresh_rate_ms: 1,
        }],
    };
    ws.send(Message::Text(
        serde_json::to_string(&msg).unwrap().into(),
    ))
    .await
    .unwrap();

    let start = Instant::now();
    let mut last = start;
    let mut count = 0u64;
    while let Some(Ok(msg)) = ws.next().await {
        match msg {
            Message::Text(t) => {
                count += 1;
                let now = Instant::now();
                let delta = now - last;
                let total = now - start;
                println!(
                    "{} delta={:?} total={} msg={}",
                    count,
                    delta,
                    total.as_millis(),
                    t.trim()
                );
                last = now;
                if count >= 50 {
                    break;
                }
            }
            Message::Close(_) => break,
            _ => {}
        }
    }
}
