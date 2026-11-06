//! Moduł 3 (Zadanie 3.4): Sniffer Warstwy 2 (`pcap`) i telemetria *in-place* (`zerocopy` v0.8).
//!
//! Ćwiczenie obejmuje:
//! - Polimorfizm `pcap::Activated` dla plików offline i kart sieciowych,
//! - Bezalokacyjne cięcie warstw `etherparse::SlicedPacket` (Ethernet II i Linux SLL),
//! - Bezpieczną transmutację struktur `zerocopy` v0.8 bez ryzyka wycieku paddingu i błędów wyrównania.

use etherparse::{SlicedPacket, TransportSlice};
use pcap::{Activated, Capture, Linktype};
use pnet::datalink::DataLinkSender;
use zerocopy::network_endian::{U16, U32};
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned};

/// 8-bajtowy nagłówek telemetrii wstrzykiwany przez sondę przed ładunkiem
#[derive(Debug, Clone, Copy, PartialEq, Eq, FromBytes, IntoBytes, KnownLayout, Immutable, Unaligned)]
#[repr(C)]
pub struct ProbeTelemetryHeader {
    pub probe_id: U16,
    pub flags: u8,
    pub reserved_pad: u8, // Zapobiega paddingowi (kluczowe dla IntoBytes!)
    pub rx_timestamp_us: U32,
}

pub fn decode_udp_payloads_from_pcap<T: Activated>(
    _cap: &mut Capture<T>,
) -> anyhow::Result<Vec<(u16, u16, Vec<u8>)>> {
    // Zadanie 3.4: Ekstrakcja ładunków UDP z ramek PCAP przez etherparse
    todo!()
}

pub fn emit_telemetry_frame_in_place(
    _tx: &mut Box<dyn DataLinkSender>,
    _probe_id: u16,
    _rx_ts_us: u32,
    _rtp_payload: &[u8],
) -> anyhow::Result<()> {
    // Zadanie 3.4: Bezalokacyjne wstrzykiwanie nagłówka telemetrii z zerocopy
    todo!()
}

pub fn parse_telemetry_frame_zero_copy(frame: &[u8]) -> anyhow::Result<(&ProbeTelemetryHeader, &[u8])> {
    ProbeTelemetryHeader::ref_from_prefix(frame)
        .map_err(|e| anyhow::anyhow!("Błąd rzutowania zerocopy: {e:?}"))
}
