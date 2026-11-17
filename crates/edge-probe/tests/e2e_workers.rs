//! Test E2E wieloworkerowego orkiestratora (`SO_REUSEPORT`, KMS, control-plane, SRTP i Sans-I/O).

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;
//
use axum::extract::Path;
use axum::http::StatusCode;
use axum::{Json, Router, routing::get};
use chrono::{Duration as ChronoDuration, Utc};
use edge_probe::control_client::ControlClient;
use edge_probe::control_plane::{
  AlertStats, AppState, create_router, init_db, spawn_alert_writer,
};
use edge_probe::kms_client::{KmsClient, KmsKeyResponse};
use edge_probe::{orchestrator, parse_ssrc};
use rtp_parser::build_test_rtp_packet;
use tokio::net::{TcpListener, UdpSocket};
use tokio::sync::{mpsc, watch};
use tokio::time::{Instant, sleep, timeout};
use webrtc_srtp::context::Context;
use webrtc_srtp::protection_profile::ProtectionProfile;

const KEY_HEX: &str = "000102030405060708090a0b0c0d0e0f";
const SALT_HEX: &str = "0e0d0c0b0a090807060504030201";

fn test_tx_context() -> Context {
  let key = hex::decode(KEY_HEX).unwrap();
  let salt = hex::decode(SALT_HEX).unwrap();
  Context::new(
    &key,
    &salt,
    ProtectionProfile::Aes128CmHmacSha1_80,
    None,
    None,
  )
  .unwrap()
}

async fn handle_kms_key(
  Path(raw_ssrc): Path<String>,
) -> Result<Json<KmsKeyResponse>, StatusCode> {
  let ssrc = parse_ssrc(&raw_ssrc).ok_or(StatusCode::BAD_REQUEST)?;
  Ok(Json(KmsKeyResponse {
    ssrc,
    master_key_hex: KEY_HEX.into(),
    master_salt_hex: SALT_HEX.into(),
    expires_at: Utc::now() + ChronoDuration::minutes(5),
  }))
}

async fn free_udp_addr() -> SocketAddr {
  let sock = UdpSocket::bind("127.0.0.1:0")
    .await
    .unwrap();
  sock.local_addr().unwrap()
}

#[tokio::test]
async fn e2e_workers_pipeline_verifies_srtp_detects_gap_and_forwards() {
  let db = init_db("sqlite::memory:")
    .await
    .unwrap();
  let (alerts_tx, alerts_rx) = mpsc::channel(64);
  let (stats_tx, stats_rx) = watch::channel(AlertStats::default());
  spawn_alert_writer(db, alerts_rx, stats_tx);

  let control_state = Arc::new(AppState {
    streams: Mutex::new(HashMap::new()),
    alerts_tx,
    stats_rx: stats_rx.clone(),
  });
  let control_listener = TcpListener::bind("127.0.0.1:0")
    .await
    .unwrap();
  let control_addr = control_listener.local_addr().unwrap();
  let control_app = create_router(control_state.clone());
  let control_srv = tokio::spawn(async move {
    let _ = axum::serve(control_listener, control_app).await;
  });

  let kms_listener = TcpListener::bind("127.0.0.1:0")
    .await
    .unwrap();
  let kms_addr = kms_listener.local_addr().unwrap();
  let kms_app = Router::new().route("/v1/keys/{ssrc}", get(handle_kms_key));
  let kms_srv = tokio::spawn(async move {
    let _ = axum::serve(kms_listener, kms_app).await;
  });

  let egress_sink = UdpSocket::bind("127.0.0.1:0")
    .await
    .unwrap();
  let egress_addr = egress_sink.local_addr().unwrap();
  let listen_addr = free_udp_addr().await;

  let kms_url = format!("http://{kms_addr}");
  let control_url = format!("http://{control_addr}");
  let kms_client = KmsClient::new(&kms_url);
  let control_client = ControlClient::new(&control_url);

  let probe_handle = tokio::spawn(async move {
    orchestrator::run_probe_workers(
      &kms_client,
      &control_client,
      listen_addr,
      egress_addr,
      2,
    )
    .await
  });

  sleep(Duration::from_millis(50)).await;

  let ssrc_a = 0x1111_2222;
  let ssrc_b = 0x3333_4444;

  let sender_a = UdpSocket::bind("127.0.0.1:0")
    .await
    .unwrap();
  let mut tx_a = test_tx_context();
  for seq in [1u16, 2, 4] {
    let rtp =
      build_test_rtp_packet(seq, (seq as u32) * 3000, ssrc_a, b"stream-a");
    let srtp = tx_a.encrypt_rtp(&rtp).unwrap();
    sender_a
      .send_to(&srtp, listen_addr)
      .await
      .unwrap();
  }
  let rtcp_a = [0x80u8, 200, 0x00, 0x01, 0x11, 0x11, 0x22, 0x22];
  let srtcp_a = tx_a.encrypt_rtcp(&rtcp_a).unwrap();
  sender_a
    .send_to(&srtcp_a, listen_addr)
    .await
    .unwrap();

  let mut tampered = tx_a
    .encrypt_rtp(&build_test_rtp_packet(5, 15000, ssrc_a, b"tampered"))
    .unwrap()
    .to_vec();
  tampered[3] ^= 0x01;
  sender_a
    .send_to(&tampered, listen_addr)
    .await
    .unwrap();

  let sender_b = UdpSocket::bind("127.0.0.1:0")
    .await
    .unwrap();
  let mut tx_b = test_tx_context();
  for seq in [10u16, 11, 12] {
    let rtp =
      build_test_rtp_packet(seq, (seq as u32) * 3000, ssrc_b, b"stream-b");
    let srtp = tx_b.encrypt_rtp(&rtp).unwrap();
    sender_b
      .send_to(&srtp, listen_addr)
      .await
      .unwrap();
  }

  let mut forwarded_count = 0usize;
  let mut buf = [0u8; 2048];
  let deadline = Instant::now() + Duration::from_secs(3);
  while forwarded_count < 7 {
    let remaining = deadline.saturating_duration_since(Instant::now());
    let (n, _) = timeout(remaining, egress_sink.recv_from(&mut buf))
      .await
      .expect("timeout oczekiwania na przekazane ramki")
      .unwrap();
    assert!(n > 0);
    forwarded_count += 1;
  }

  let poll_deadline = Instant::now() + Duration::from_secs(3);
  loop {
    let registered_both = {
      let streams = control_state.streams.lock().unwrap();
      streams.contains_key(&ssrc_a) && streams.contains_key(&ssrc_b)
    };
    let alerts_ok = stats_rx.borrow().total_alerts >= 2;
    if registered_both && alerts_ok {
      break;
    }
    assert!(
      Instant::now() < poll_deadline,
      "nie zarejestrowano obu strumieni lub alertów na czas"
    );
    sleep(Duration::from_millis(25)).await;
  }

  assert_eq!(forwarded_count, 7);
  assert!(stats_rx.borrow().total_alerts >= 2);

  probe_handle.abort();
  control_srv.abort();
  kms_srv.abort();
}
