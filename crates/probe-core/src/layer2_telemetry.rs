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
    cap: &mut Capture<T>,
) -> anyhow::Result<Vec<(u16, u16, Vec<u8>)>> {
    let linktype = cap.get_datalink();
    let mut results = Vec::new();

    while let Ok(pkt) = cap.next_packet() {
        let sliced = match linktype {
            Linktype::ETHERNET => SlicedPacket::from_ethernet(pkt.data)?,
            Linktype(113) if pkt.data.len() >= 16 => SlicedPacket::from_ip(&pkt.data[16..])?, // DLT_LINUX_SLL
            _ => continue,
        };

        if let Some(TransportSlice::Udp(udp)) = sliced.transport {
            results.push((udp.source_port(), udp.destination_port(), udp.payload().to_vec()));
        }
    }
    Ok(results)
}

pub fn emit_telemetry_frame_in_place(
    tx: &mut Box<dyn DataLinkSender>,
    probe_id: u16,
    rx_ts_us: u32,
    rtp_payload: &[u8],
) -> anyhow::Result<()> {
    let total_len = std::mem::size_of::<ProbeTelemetryHeader>() + rtp_payload.len();
    tx.build_and_send(1, total_len, &mut |frame_buf: &mut [u8]| {
        let (hdr_bytes, body_bytes) = frame_buf.split_at_mut(std::mem::size_of::<ProbeTelemetryHeader>());
        let hdr = ProbeTelemetryHeader::mut_from_bytes(hdr_bytes)
            .expect("Rozmiar dopasowany dokładnie przez split_at_mut");
        hdr.probe_id = U16::new(probe_id);
        hdr.flags = 0b0000_0001;
        hdr.reserved_pad = 0;
        hdr.rx_timestamp_us = U32::new(rx_ts_us);
        body_bytes.copy_from_slice(rtp_payload);
    })
    .ok_or_else(|| anyhow::anyhow!("Brak bufora w kanale DataLinkSender"))??;
    Ok(())
}

pub fn parse_telemetry_frame_zero_copy(frame: &[u8]) -> anyhow::Result<(&ProbeTelemetryHeader, &[u8])> {
    ProbeTelemetryHeader::ref_from_prefix(frame)
        .map_err(|e| anyhow::anyhow!("Błąd rzutowania zerocopy: {e:?}"))
}


