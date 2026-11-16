//! Zadanie 11: deterministyczna symulacja usterek sieciowych w `turmoil`.

use std::time::{Duration, Instant};
//
use bytes::Bytes;
use chrono::{Duration as ChronoDuration, Utc};
use edge_probe::kms_client::KmsKeyResponse;
use edge_probe::rtp_session::{ProtocolOutput, SansIoRtpSession};
use edge_probe::srtp_layer::SrtpVerifier;
use rtp_parser::{
  PacketClass, build_test_rtp_packet, classify_peeked_packet, parse_rtp_header,
};
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
    .min_message_latency(Duration::from_millis(5))
    .max_message_latency(Duration::from_millis(20))
    .build();

  sim.host("probe", || async {
    let sock = UdpSocket::bind("0.0.0.0:5004").await?;
    let mut verifier = SrtpVerifier::from_kms_key(&test_kms_key()).unwrap();
    let start = Instant::now();
    let mut session =
      SansIoRtpSession::new(0x1000, start, Duration::from_secs(5));
    let mut buf = [0u8; 2048];
    let mut rejected_replays = 0usize;
    let mut forwarded_rtcp = 0usize;

    // Nadawca wysyła 4 pakiety SRTP (65534, 0, duplikat 0, 1) oraz 1 SRTCP; 65535 ginie.
    for _ in 0..5 {
      let (n, _) = sock.recv_from(&mut buf).await?;
      if classify_peeked_packet(&buf[..2]) == Some(PacketClass::Rtcp) {
        if verifier
          .verify_rtcp_packet(&buf[..n])
          .is_ok()
        {
          forwarded_rtcp += 1;
        }
        continue;
      }
      if verifier
        .verify_packet(&buf[..n])
        .is_err()
      {
        rejected_replays += 1;
        continue;
      }
      let frame = Bytes::copy_from_slice(&buf[..n]);
      let (hdr, payload) = parse_rtp_header(frame).unwrap();
      session.handle_rtp(
        hdr.version,
        hdr.ssrc,
        hdr.sequence_number,
        payload.len(),
      );
    }

    session.handle_timeout(start + Duration::from_secs(5));

    let mut nack_missing = None;
    let mut rr_ext_seq = None;
    while let Some(out) = session.poll_output() {
      match out {
        ProtocolOutput::SendRtcpNack { missing_seqs, .. } => {
          nack_missing = Some(missing_seqs);
        }
        ProtocolOutput::SendRtcpReceiverReport {
          extended_highest_seq,
          ..
        } => {
          rr_ext_seq = Some(extended_highest_seq);
        }
        ProtocolOutput::ForwardRtp { .. } => {}
      }
    }

    assert_eq!(forwarded_rtcp, 1);
    assert_eq!(rejected_replays, 1);
    assert_eq!(session.roc, 1);
    assert_eq!(nack_missing, Some(vec![65535]));
    assert_eq!(rr_ext_seq, Some((1 << 16) | 1));
    Ok(())
  });

  sim.client("sender", async {
    let sock = UdpSocket::bind("0.0.0.0:0").await?;
    let mut tx = test_tx_context();

    let p_65534 = build_test_srtp_packet(&mut tx, 65534);
    let p_65535 = build_test_srtp_packet(&mut tx, 65535);
    let p_0 = build_test_srtp_packet(&mut tx, 0);
    let p_1 = build_test_srtp_packet(&mut tx, 1);
    let srtcp_sr = build_test_srtcp_packet(&mut tx);

    sock
      .send_to(&p_65534, "probe:5004")
      .await?;
    tokio::time::sleep(Duration::from_millis(25)).await;

    turmoil::partition_oneway("sender", "probe");
    let _ = sock
      .send_to(&p_65535, "probe:5004")
      .await;
    tokio::time::sleep(Duration::from_millis(25)).await;

    turmoil::repair("sender", "probe");
    sock.send_to(&p_0, "probe:5004").await?;
    sock.send_to(&p_0, "probe:5004").await?; // duplikat (Replay Protection)
    sock.send_to(&p_1, "probe:5004").await?;
    sock
      .send_to(&srtcp_sr, "probe:5004")
      .await?;
    Ok(())
  });

  sim.run()
}
