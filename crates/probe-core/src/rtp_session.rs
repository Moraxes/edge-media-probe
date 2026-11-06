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
        // Zadanie 3.2: Zaimplementuj maszynę stanów przetwarzania pakietu RTP (Sans-I/O):
        // Wymagania:
        // 1. Sprawdź, czy `version == 2` oraz `payload_len > 0`.
        // 2. Jeśli to pierwszy odebrany pakiet, ustaw `self.media_ssrc = ssrc`.
        // 3. Sprawdź, czy ssrc zgadza się z `self.media_ssrc`.
        // 4. Wykorzystaj arytmetykę modularną: `let delta = seq.wrapping_sub(prev)`:
        //    - delta == 0: duplikat, zignoruj.
        //    - delta < 0x8000: nowy pakiet w sekwencji.
        //      - Jeśli seq < prev: nastąpiło zawinięcie 16-bitowego numeru (65535 -> 0), zwiększ `self.roc += 1`.
        //      - Jeśli delta > 1: wykryto lukę! Oblicz brakujące numery sekwencyjne:
        //        `let missing: Vec<u16> = (1..delta).map(|off| prev.wrapping_add(off)).collect();`
        //        Zwiększ `self.lost_packets += missing.len()` i wyemituj `ProtocolOutput::SendRtcpNack`.
        //      - Zaktualizuj `self.highest_seq = Some(seq)`.
        //    - delta >= 0x8000: opóźniony pakiet (out-of-order), zredukuj `self.lost_packets.saturating_sub(1)`.
        // 5. Dodaj zdarzenie `ProtocolOutput::ForwardRtp { ssrc, seq, roc: self.roc }` do `self.out_queue`.
        todo!("Zadanie 3.2: Zaimplementuj handle_rtp z arytmetyką wrapping_sub i ROC")
    }

    pub fn poll_timeout(&self) -> Instant {
        self.next_rr_deadline
    }

    pub fn handle_timeout(&mut self, _now: Instant) {
        // Zadanie 3.2: Zaimplementuj okresowe generowanie raportu odbiorcy RTCP (Receiver Report):
        // Wymagania:
        // 1. Sprawdź, czy `now >= self.next_rr_deadline`.
        // 2. Jeśli ustalono najwyższy numer sekwencyjny (`self.highest_seq`):
        //    - Oblicz 32-bitowy rozszerzony numer sekwencji: `(self.roc << 16) | u32::from(highest)`.
        //    - Wstaw do `self.out_queue` zdarzenie `ProtocolOutput::SendRtcpReceiverReport`.
        // 3. Przesuń termin raportu: `self.next_rr_deadline = now + self.rr_interval`.
        todo!("Zadanie 3.2: Zaimplementuj handle_timeout generujący RTCP Receiver Report")
    }

    pub fn poll_output(&mut self) -> Option<ProtocolOutput> {
        self.out_queue.pop_front()
    }
}

pub fn sync_av_pair_metrics(_streams: &mut [StreamMetrics], _audio_idx: usize, _video_idx: usize) -> bool {
    // Zadanie 3.2: Zaimplementuj jednoczesną modyfikację dwóch elementów w tablicy strumieni:
    // Wymagania:
    // 1. Użyj metody `streams.get_disjoint_mut([audio_idx, video_idx])` (dostępnej od Rust 1.86+).
    // 2. W przypadku sukcesu zwiększ liczniki `packets_ok` w strumieniu audio i wideo o 1 i zwróć `true`.
    // 3. W przypadku indeksów nakładających się lub poza zakresem zwróć `false`.
    todo!("Zadanie 3.2: Zaimplementuj sync_av_pair_metrics z slice::get_disjoint_mut")
}
