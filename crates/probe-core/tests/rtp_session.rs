use probe_core::rtp_session::{
    sync_av_pair_metrics, ProtocolOutput, SansIoRtpSession, StreamMetrics,
};
use std::time::{Duration, Instant};

#[test]
fn test_wrap_around_nack_and_rtcp_rr_without_sleep() {
    let t0 = Instant::now();
    let mut session = SansIoRtpSession::new(0x1234_5678, t0, Duration::from_secs(1));

    session.handle_rtp(2, 0x1234_5678, 65534, 160);
    assert_eq!(
        session.poll_output(),
        Some(ProtocolOutput::ForwardRtp { ssrc: 0x1234_5678, seq: 65534, roc: 0 })
    );

    session.handle_rtp(2, 0x1234_5678, 1, 160);

    assert_eq!(
        session.poll_output(),
        Some(ProtocolOutput::SendRtcpNack {
            media_ssrc: 0x1234_5678,
            missing_seqs: vec![65535, 0],
        })
    );
    assert_eq!(
        session.poll_output(),
        Some(ProtocolOutput::ForwardRtp { ssrc: 0x1234_5678, seq: 1, roc: 1 })
    );

    let t1 = session.poll_timeout();
    session.handle_timeout(t1);
    assert_eq!(
        session.poll_output(),
        Some(ProtocolOutput::SendRtcpReceiverReport {
            media_ssrc: 0x1234_5678,
            extended_highest_seq: (1 << 16) | 1,
            lost_packets: 2,
        })
    );
}

#[test]
fn test_sync_av_pair_metrics_disjoint() {
    let mut streams = [
        StreamMetrics { ssrc: 1, packets_ok: 10, nacks_triggered: 0 },
        StreamMetrics { ssrc: 2, packets_ok: 20, nacks_triggered: 0 },
    ];

    assert!(sync_av_pair_metrics(&mut streams, 0, 1));
    assert_eq!(streams[0].packets_ok, 11);
    assert_eq!(streams[1].packets_ok, 21);

    // Nakładające się indeksy muszą zostać bezpiecznie odrzucone:
    assert!(!sync_av_pair_metrics(&mut streams, 0, 0));
}

