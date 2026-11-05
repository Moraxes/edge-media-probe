use edge_media_probe::rtp_session::{ProtocolOutput, SansIoRtpSession};
use edge_media_probe::srtp_layer::create_srtp_pair;
use edge_media_probe::udp_ingress::{build_test_rtp_packet, parse_rtp_header};
use std::time::{Duration, Instant};

#[test]
fn test_srtp_header_stays_cleartext_and_authenticated() -> anyhow::Result<()> {
    let (mut tx_ctx, mut rx_ctx) = create_srtp_pair()?;
    let clear_rtp = build_test_rtp_packet(100, 90_000, 0xDEAD_BEEF, b"TajnaKlatkaOpus");

    let srtp_packet = tx_ctx.encrypt_rtp(&clear_rtp)?;
    assert_eq!(srtp_packet.len(), clear_rtp.len() + 10);
    assert_eq!(&srtp_packet[..12], &clear_rtp[..12], "Nagłówek RTP pozostaje jawny");
    assert_ne!(&srtp_packet[12..clear_rtp.len()], &clear_rtp[12..], "Ładunek jest zaszyfrowany");

    let decrypted = rx_ctx.decrypt_rtp(&srtp_packet)?;
    assert_eq!(&decrypted[..], &clear_rtp[..]);
    Ok(())
}

#[test]
fn test_turmoil_partition_triggers_rtcp_nack() -> turmoil::Result {
    let mut sim = turmoil::Builder::new()
        .min_message_latency(Duration::ZERO)
        .max_message_latency(Duration::ZERO)
        .build();

    sim.host("probe", || async {
        let sock = turmoil::net::UdpSocket::bind("0.0.0.0:5004").await?;
        let (_, mut rx_srtp) = create_srtp_pair().unwrap();
        let mut session = SansIoRtpSession::new(0xABCD_0001, Instant::now(), Duration::from_secs(1));
        let mut buf = [0u8; 1500];

        for _ in 0..2 {
            let (n, src) = sock.recv_from(&mut buf).await?;
            let clear = rx_srtp.decrypt_rtp(&buf[..n]).expect("Weryfikacja SRTP HMAC");
            let (hdr, payload) = parse_rtp_header(clear.into()).unwrap();
            session.handle_rtp(hdr.version, hdr.ssrc, hdr.sequence_number, payload.len());

            while let Some(out) = session.poll_output() {
                if let ProtocolOutput::SendRtcpNack { missing_seqs, .. } = out {
                    let report: Vec<u8> = missing_seqs.iter().flat_map(|s| s.to_be_bytes()).collect();
                    sock.send_to(&report, src).await?;
                }
            }
        }
        Ok(())
    });

    sim.client("sender", async {
        let sock = turmoil::net::UdpSocket::bind("0.0.0.0:0").await?;
        let (mut tx_srtp, _) = create_srtp_pair().unwrap();

        let p10 = tx_srtp.encrypt_rtp(&build_test_rtp_packet(10, 1000, 0xABCD_0001, b"frame10")).unwrap();
        sock.send_to(&p10, "probe:5004").await?;
        tokio::time::sleep(Duration::from_millis(10)).await;

        turmoil::partition_oneway("sender", "probe");
        for seq in [11u16, 12u16] {
            let pkt = tx_srtp.encrypt_rtp(&build_test_rtp_packet(seq, 2000, 0xABCD_0001, b"lost")).unwrap();
            sock.send_to(&pkt, "probe:5004").await?;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;

        turmoil::repair("sender", "probe");
        let p13 = tx_srtp.encrypt_rtp(&build_test_rtp_packet(13, 3000, 0xABCD_0001, b"frame13")).unwrap();
        sock.send_to(&p13, "probe:5004").await?;

        let mut nack_buf = [0u8; 16];
        let (n, _) = sock.recv_from(&mut nack_buf).await?;
        assert_eq!(&nack_buf[..n], &[0, 11, 0, 12]);
        Ok(())
    });

    sim.run()
}

