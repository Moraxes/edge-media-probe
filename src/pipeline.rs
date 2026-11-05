//! Moduł 3 (Zadanie 3.5 / Capstone): Zintegrowana pętla przetwarzania pakietu.
//!
//! Spina:
//! - Klasyfikację `rtcp-mux` (Zadanie 3.1),
//! - Deszyfrowanie SRTP i weryfikację HMAC (Zadanie 3.3),
//! - Maszynę stanów *Sans-I/O* (Zadanie 3.2),
//! - Gorącą ścieżkę metryk i zimną ścieżkę anomalii (Zadania 2.2 & 2.3),
//! - Asynchroniczny zrzut ramki na dysk przez `AsyncFileSink` w stylu `io_uring` (Zadanie 1.4).

use crate::observability::ProbeDataplaneMetrics;
use crate::pcap_sink::AsyncFileSink;
use crate::rtp_session::{ProtocolOutput, SansIoRtpSession};
use crate::udp_ingress::{classify_peeked_packet, parse_rtp_header, PacketClass};
use metrics::{counter, histogram};
use std::io;
use tracing::{info, warn};

pub async fn process_srtp_datagram_with_observability_and_uring<S: AsyncFileSink>(
    raw_datagram: &[u8],
    rx_srtp: &mut webrtc_srtp::context::Context,
    rtp_session: &mut SansIoRtpSession,
    pcap_sink: &mut S,
    disk_offset: &mut u64,
    mut ring_buf: Vec<u8>,
) -> io::Result<Vec<u8>> {
    // 1. Klasyfikacja rtcp-mux
    let Some(class) = classify_peeked_packet(raw_datagram) else {
        counter!("probe_packets_dropped_total", "reason" => "invalid_rtp_version").increment(1);
        return Ok(ring_buf);
    };

    if class == PacketClass::Rtcp {
        counter!("probe_packets_total", "kind" => "rtcp").increment(1);
        return Ok(ring_buf);
    }

    // 2. Weryfikacja HMAC i deszyfrowanie SRTP (z obsługą czystego RTP w celach laboratoryjnych / ffmpeg / mock)
    let decrypted = match rx_srtp.decrypt_rtp(raw_datagram) {
        Ok(clear) => clear,
        Err(err) => {
            if let Ok((_hdr, payload)) = parse_rtp_header(bytes::Bytes::copy_from_slice(raw_datagram)) {
                if !payload.is_empty() {
                    bytes::Bytes::copy_from_slice(raw_datagram)
                } else {
                    ProbeDataplaneMetrics::on_srtp_auth_error();
                    warn!(?err, len = raw_datagram.len(), "Odrzucono pakiet SRTP (błąd HMAC lub Replay)");
                    return Ok(ring_buf);
                }
            } else {
                ProbeDataplaneMetrics::on_srtp_auth_error();
                warn!(?err, len = raw_datagram.len(), "Odrzucono pakiet SRTP (błąd HMAC lub Replay)");
                return Ok(ring_buf);
            }
        }
    };

    // 3. Gorąca ścieżka metryk
    counter!("probe_packets_total", "kind" => "rtp").increment(1);
    histogram!("probe_packet_size_bytes").record(decrypted.len() as f64);

    // 4. Maszyna stanów Sans-I/O
    if let Ok((hdr, payload)) = parse_rtp_header(decrypted) {
        rtp_session.handle_rtp(hdr.version, hdr.ssrc, hdr.sequence_number, payload.len());

        while let Some(output) = rtp_session.poll_output() {
            match output {
                ProtocolOutput::ForwardRtp { .. } => {}
                ProtocolOutput::SendRtcpNack { media_ssrc, missing_seqs } => {
                    ProbeDataplaneMetrics::on_rtp_gap_detected(missing_seqs.len());
                    warn!(
                        ssrc = %format!("{media_ssrc:#010x}"),
                        missing_count = missing_seqs.len(),
                        ?missing_seqs,
                        "Wykryto lukę w strumieniu RTP – zrzut ramki przez AsyncFileSink"
                    );

                    ring_buf.clear();
                    ring_buf.extend_from_slice(raw_datagram);
                    let (res, returned_buf) = pcap_sink.write_all_at(*disk_offset, ring_buf).await;
                    ring_buf = returned_buf;
                    *disk_offset += res? as u64;
                }
                ProtocolOutput::SendRtcpReceiverReport { media_ssrc, extended_highest_seq, lost_packets } => {
                    ProbeDataplaneMetrics::on_rtcp_receiver_report(lost_packets, extended_highest_seq);
                    info!(
                        ssrc = %format!("{media_ssrc:#010x}"),
                        extended_highest_seq,
                        lost_packets,
                        "Wygenerowano raport RTCP Receiver Report (RR)"
                    );
                }
            }
        }
    }

    Ok(ring_buf)
}

