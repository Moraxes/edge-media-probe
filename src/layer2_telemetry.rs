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

#[cfg(test)]
mod tests {
    use super::*;
    use etherparse::PacketBuilder;
    use pnet::datalink::{self, DataLinkReceiver};

    #[test]
    fn test_pcap_dead_savefile_and_generic_activated_sniffer() -> anyhow::Result<()> {
        let tmp_pcap = std::env::temp_dir().join("edge_probe_test.pcap");

        let mut raw_frame = Vec::with_capacity(128);
        PacketBuilder::ethernet2([0x02, 0, 0, 0, 0, 1], [0x02, 0, 0, 0, 0, 2])
            .ipv4([10, 0, 0, 1], [10, 0, 0, 2], 64)
            .udp(5004, 6000)
            .write(&mut raw_frame, &[0x80, 96, 0x00, 0x2A, 0, 0, 0, 1, 0, 0, 0, 2])?;

        {
            let dead = Capture::dead(Linktype::ETHERNET)?;
            let mut savefile = dead.savefile(&tmp_pcap)?;
            let header = pcap::PacketHeader {
                ts: libc::timeval { tv_sec: 1_700_000_000, tv_usec: 0 },
                caplen: raw_frame.len() as u32,
                len: raw_frame.len() as u32,
            };
            savefile.write(&pcap::Packet { header: &header, data: &raw_frame });
            savefile.flush()?;
        }

        let mut offline = Capture::from_file(&tmp_pcap)?;
        let packets = decode_udp_payloads_from_pcap(&mut offline)?;
        assert_eq!(packets.len(), 1);
        assert_eq!(packets[0].0, 5004);
        assert_eq!(packets[0].1, 6000);
        let _ = std::fs::remove_file(&tmp_pcap);
        Ok(())
    }

    #[test]
    fn test_pnet_dummy_in_place_zerocopy_telemetry() -> anyhow::Result<()> {
        let iface = datalink::dummy::dummy_interface(0);
        let cfg = datalink::dummy::Config::default();
        let (mut tx, _rx): (Box<dyn DataLinkSender>, Box<dyn DataLinkReceiver>) =
            match datalink::dummy::channel(&iface, cfg)? {
                datalink::Channel::Ethernet(tx, rx) => (tx, rx),
                _ => anyhow::bail!("Nieoczekiwany kanał"),
            };

        // Weryfikacja budowy ramki in-place w kanale sterownika DataLinkSender
        emit_telemetry_frame_in_place(&mut tx, 77, 123_456_789, b"RTP_FRAME")?;

        // Weryfikacja parsowania zerocopy
        let mut raw_frame = vec![0u8; std::mem::size_of::<ProbeTelemetryHeader>() + 9];
        let (hdr_part, body_part) = raw_frame.split_at_mut(std::mem::size_of::<ProbeTelemetryHeader>());
        let hdr = ProbeTelemetryHeader::mut_from_bytes(hdr_part).unwrap();
        hdr.probe_id = U16::new(77);
        hdr.flags = 0b0000_0001;
        hdr.reserved_pad = 0;
        hdr.rx_timestamp_us = U32::new(123_456_789);
        body_part.copy_from_slice(b"RTP_FRAME");

        let (hdr_parsed, payload) = parse_telemetry_frame_zero_copy(&raw_frame)?;
        assert_eq!(hdr_parsed.probe_id.get(), 77);
        assert_eq!(hdr_parsed.rx_timestamp_us.get(), 123_456_789);
        assert_eq!(payload, b"RTP_FRAME");
        Ok(())
    }
}

