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
    // Zadanie 3.4: Zaimplementuj ekstrakcję ładunków UDP z przechwyconych pakietów:
    // Wymagania:
    // 1. Sprawdź warstwę łącza danych za pomocą `cap.get_datalink()`.
    // 2. W pętli `while let Ok(pkt) = cap.next_packet()` rozróżnij `Linktype::ETHERNET`
    //    i `Linktype(113)` (Linux SLL).
    // 3. Użyj `etherparse::SlicedPacket::from_ethernet` lub `from_ip`.
    // 4. Jeśli pakiet zawiera warstwę UDP (`sliced.transport == Some(TransportSlice::Udp(udp))`),
    //    zapisz krotkę `(udp.source_port(), udp.destination_port(), udp.payload().to_vec())`.
    // 5. Zwróć wektor odebranych pakietów.
    todo!("Zadanie 3.4: Zaimplementuj decode_udp_payloads_from_pcap z etherparse")
}

pub fn emit_telemetry_frame_in_place(
    _tx: &mut Box<dyn DataLinkSender>,
    _probe_id: u16,
    _rx_ts_us: u32,
    _rtp_payload: &[u8],
) -> anyhow::Result<()> {
    // Zadanie 3.4: Zaimplementuj bezalokacyjne wstrzykiwanie nagłówka telemetrii za pomocą zerocopy:
    // Wymagania:
    // 1. Oblicz całkowitą długość: `std::mem::size_of::<ProbeTelemetryHeader>() + rtp_payload.len()`.
    // 2. Wywołaj `tx.build_and_send(1, total_len, &mut |frame_buf| { ... })`.
    // 3. Podziel bufor ramki na część nagłówka i ciała (`frame_buf.split_at_mut(...)`).
    // 4. Użyj `ProbeTelemetryHeader::mut_from_bytes(hdr_bytes)` do bezpośredniej edycji w buforze jądra/karty.
    // 5. Ustaw pola `probe_id`, `flags`, `reserved_pad = 0`, `rx_timestamp_us`.
    // 6. Skopiuj `rtp_payload` do części bufora ładunku.
    todo!("Zadanie 3.4: Zaimplementuj emit_telemetry_frame_in_place z zerocopy")
}

pub fn parse_telemetry_frame_zero_copy(frame: &[u8]) -> anyhow::Result<(&ProbeTelemetryHeader, &[u8])> {
    // Zadanie 3.4: Zaimplementuj bezalokacyjne odczytanie nagłówka telemetrii ze strumienia:
    // Wymagania:
    // 1. Użyj metody `ProbeTelemetryHeader::ref_from_prefix(frame)`.
    // 2. Zwróć referencję do nagłówka oraz pozostały wycinek bufora.
    ProbeTelemetryHeader::ref_from_prefix(frame)
        .map_err(|e| anyhow::anyhow!("Błąd rzutowania zerocopy: {e:?}"))
}
