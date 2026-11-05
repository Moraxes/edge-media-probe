//! Moduł 3 (Zadanie 3.2): Maszyna stanów *Sans-I/O* dla RTP i sprzężenie zwrotne RTCP.
//!
//! Ćwiczenie obejmuje:
//! - Detekcję luk sekwencji z arytmetyką modularną `wrapping_sub` i licznikiem przepełnień `ROC`,
//! - Generowanie zdarzeń `ProtocolOutput::SendRtcpNack` i `ProtocolOutput::SendReceiverReport`,
//! - Zastosowanie `slice::get_disjoint_mut` (Rust 1.86+) do mutacji wielu strumieni w tablicy.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtocolOutput {
    ForwardRtp { ssrc: u32, seq: u16, roc: u32 },
    SendRtcpNack { media_ssrc: u32, missing_seqs: Vec<u16> },
    SendRtcpReceiverReport {
        media_ssrc: u32,
        extended_highest_seq: u32,
        lost_packets: u32,
    },
}

#[derive(Debug, Default)]
pub struct StreamMetrics {
    pub ssrc: u32,
    pub packets_ok: u64,
    pub nacks_triggered: u64,
}

pub struct SansIoRtpSession {
    media_ssrc: u32,
    highest_seq: Option<u16>,
    roc: u32,
    received_packets: u32,
    lost_packets: u32,
    next_rr_deadline: Instant,
    rr_interval: Duration,
    out_queue: VecDeque<ProtocolOutput>,
}

impl SansIoRtpSession {
    pub fn new(media_ssrc: u32, now: Instant, rr_interval: Duration) -> Self {
        Self {
            media_ssrc,
            highest_seq: None,
            roc: 0,
            received_packets: 0,
            lost_packets: 0,
            next_rr_deadline: now + rr_interval,
            rr_interval,
            out_queue: VecDeque::new(),
        }
    }

    pub fn handle_rtp(&mut self, version: u8, ssrc: u32, seq: u16, payload_len: usize) {
        if version == 2 && ssrc == self.media_ssrc && payload_len > 0 {
            self.received_packets += 1;

            if let Some(prev) = self.highest_seq {
                let delta = seq.wrapping_sub(prev);
                if delta == 0 {
                    return; // Duplikat
                } else if delta < 0x8000 {
                    if seq < prev {
                        self.roc += 1; // Zawinięcie 65535 -> 0
                    }
                    if delta > 1 {
                        let missing: Vec<u16> = (1..delta)
                            .map(|offset| prev.wrapping_add(offset))
                            .collect();
                        self.lost_packets += missing.len() as u32;
                        self.out_queue.push_back(ProtocolOutput::SendRtcpNack {
                            media_ssrc: self.media_ssrc,
                            missing_seqs: missing,
                        });
                    }
                    self.highest_seq = Some(seq);
                } else {
                    self.lost_packets = self.lost_packets.saturating_sub(1);
                }
            } else {
                self.highest_seq = Some(seq);
            }

            self.out_queue.push_back(ProtocolOutput::ForwardRtp {
                ssrc,
                seq,
                roc: self.roc,
            });
        }
    }

    pub fn poll_timeout(&self) -> Instant {
        self.next_rr_deadline
    }

    pub fn handle_timeout(&mut self, now: Instant) {
        if now >= self.next_rr_deadline {
            if let Some(highest) = self.highest_seq {
                let ext_seq = (self.roc << 16) | u32::from(highest);
                self.out_queue.push_back(ProtocolOutput::SendRtcpReceiverReport {
                    media_ssrc: self.media_ssrc,
                    extended_highest_seq: ext_seq,
                    lost_packets: self.lost_packets,
                });
            }
            self.next_rr_deadline = now + self.rr_interval;
        }
    }

    pub fn poll_output(&mut self) -> Option<ProtocolOutput> {
        self.out_queue.pop_front()
    }
}

pub fn sync_av_pair_metrics(streams: &mut [StreamMetrics], audio_idx: usize, video_idx: usize) -> bool {
    if let Ok([audio, video]) = streams.get_disjoint_mut([audio_idx, video_idx]) {
        audio.packets_ok += 1;
        video.packets_ok += 1;
        true
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}

