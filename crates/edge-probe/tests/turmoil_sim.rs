//! Zadanie 11: deterministyczna symulacja usterek sieciowych w `turmoil`.

use std::time::Duration;
//
use chrono::{Duration as ChronoDuration, Utc};
use edge_probe::kms_client::KmsKeyResponse;
use rtp_parser::build_test_rtp_packet;
use turmoil::net::UdpSocket;
use webrtc_srtp::context::Context;
use webrtc_srtp::protection_profile::ProtectionProfile;

const KEY_HEX: &str = "000102030405060708090a0b0c0d0e0f";
const SALT_HEX: &str = "0e0d0c0b0a090807060504030201";

fn test_kms_key() -> KmsKeyResponse {
  KmsKeyResponse {
    ssrc: 0x1000,
    master_key_hex: KEY_HEX.into(),
    master_salt_hex: SALT_HEX.into(),
    expires_at: Utc::now() + ChronoDuration::minutes(5),
  }
}

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

fn build_test_srtp_packet(tx: &mut Context, seq: u16) -> Vec<u8> {
  let rtp = build_test_rtp_packet(seq, (seq as u32) * 3000, 0x1000, b"frame");
  tx.encrypt_rtp(&rtp).unwrap().to_vec()
}

fn build_test_srtcp_packet(tx: &mut Context) -> Vec<u8> {
  let rtcp = [0x80u8, 200, 0x00, 0x01, 0x00, 0x00, 0x10, 0x00];
  tx.encrypt_rtcp(&rtcp).unwrap().to_vec()
}

#[test]
fn turmoil_partition_and_replay_sim() -> turmoil::Result {
  let mut sim = turmoil::Builder::new()
    .min_message_latency(Duration::ZERO)
    .max_message_latency(Duration::ZERO)
    .build();

  sim.host("probe", || async {
    let _sock = UdpSocket::bind("0.0.0.0:5004").await?;
    // Zadanie 11: odbierz pakiety SRTP/SRTCP, zweryfikuj je przez SrtpVerifier
    // oraz przekaż RTP do SansIoRtpSession, sprawdzając roc, NACK i RR.
    Ok(())
  });

  sim.client("sender", async {
    let sock = UdpSocket::bind("0.0.0.0:0").await?;
    let mut _tx = test_tx_context();
    turmoil::partition_oneway("sender", "probe");
    let _ = sock.send_to(b"...", "probe:5004").await;
    turmoil::repair("sender", "probe");
    Ok(())
  });

  sim.run()
}
