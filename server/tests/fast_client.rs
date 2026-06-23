use humours_server::protocol::{ClientMessage, MetricSubscription};
use std::sync::Arc;
use std::time::Instant;

use futures_util::{SinkExt, StreamExt};
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{DigitallySignedStruct, Error};
use tokio_tungstenite::{Connector, connect_async_tls_with_config, tungstenite::Message};

#[derive(Debug)]
struct NoVerifier;

impl ServerCertVerifier for NoVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, Error> {
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        rustls::crypto::aws_lc_rs::default_provider()
            .signature_verification_algorithms
            .supported_schemes()
    }
}

#[tokio::test]
async fn fast_subscribe_client() {
    let url = "wss://localhost:8443/ws?token=dev-token";

    let config = Arc::new(
        rustls::ClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(NoVerifier))
            .with_no_client_auth(),
    );

    let (mut ws, _) =
        connect_async_tls_with_config(url, None, false, Some(Connector::Rustls(config)))
            .await
            .expect("connect");

    let msg = ClientMessage::Subscribe {
        metrics: vec![MetricSubscription {
            id: "cpu.usage".into(),
            refresh_rate_ms: 1,
        }],
    };
    ws.send(Message::Text(serde_json::to_string(&msg).unwrap().into()))
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
