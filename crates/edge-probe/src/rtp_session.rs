//! Maszyna stanów *Sans-I/O* dla ciągłości sekwencji RTP oraz raportowania RTCP.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtocolOutput {
    ForwardRtp {
        ssrc: u32,
        seq: u16,
        roc: u32,
    },
    SendRtcpNack {
        media_ssrc: u32,
        missing_seqs: Vec<u16>,
    },
    SendRtcpReceiverReport {
        media_ssrc: u32,
        extended_highest_seq: u32,
        received_packets: u64,
        lost_packets: u64,
    },
}

/// Zadanie 9: maszyna stanów *Sans-I/O* sesji RTP/RTCP.
pub struct SansIoRtpSession {
    pub media_ssrc: u32,
    pub highest_seq: Option<u16>,
    pub roc: u32,
    pub received_packets: u64,
    pub lost_packets: u64,
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

    /// Zadanie 9: obsłuż przychodzący nagłówek RTP (warunek wersji i SSRC,
    /// `wrapping_sub`, aktualizacja `roc`, wykrywanie luk oraz emisja `SendRtcpNack` i `ForwardRtp`).
    pub fn handle_rtp(&mut self, _version: u8, _ssrc: u32, _seq: u16, _payload_len: usize) {
        todo!()
    }

    /// Zadanie 9: po przekroczeniu `next_rr_deadline` przesuń termin o `rr_interval`
    /// i wyemituj `ProtocolOutput::SendRtcpReceiverReport`.
    pub fn handle_timeout(&mut self, _now: Instant) {
        todo!()
    }

    pub fn poll_output(&mut self) -> Option<ProtocolOutput> {
        self.out_queue.pop_front()
    }
}
