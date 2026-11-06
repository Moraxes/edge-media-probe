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
    pub media_ssrc: u32,
    pub highest_seq: Option<u16>,
    pub roc: u32,
    pub received_packets: u32,
    pub lost_packets: u32,
    pub next_rr_deadline: Instant,
    pub rr_interval: Duration,
    pub out_queue: VecDeque<ProtocolOutput>,
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

    pub fn handle_rtp(&mut self, _version: u8, _ssrc: u32, _seq: u16, _payload_len: usize) {
        // Zadanie 3.2: Maszyna stanów Sans-I/O z wrapping_sub i licznikiem ROC
        todo!()
    }

    pub fn poll_timeout(&self) -> Instant {
        self.next_rr_deadline
    }

    pub fn handle_timeout(&mut self, _now: Instant) {
        // Zadanie 3.2: Okresowe generowanie raportu RTCP Receiver Report
        todo!()
    }

    pub fn poll_output(&mut self) -> Option<ProtocolOutput> {
        self.out_queue.pop_front()
    }
}

pub fn sync_av_pair_metrics(_streams: &mut [StreamMetrics], _audio_idx: usize, _video_idx: usize) -> bool {
    // Zadanie 3.2: Bezpieczna modyfikacja dwóch strumieni z slice::get_disjoint_mut
    todo!()
}
