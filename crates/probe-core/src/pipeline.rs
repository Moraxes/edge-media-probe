//! Moduł 3 (Zadanie 3.5 / Capstone): Zintegrowana pętla przetwarzania pakietu.
//!
//! Spina:
//! - Klasyfikację `rtcp-mux` (Zadanie 3.1),
//! - Deszyfrowanie SRTP i weryfikację HMAC (Zadanie 3.3),
//! - Maszynę stanów *Sans-I/O* (Zadanie 3.2),
//! - Gorącą ścieżkę metryk i zimną ścieżkę anomalii (Zadania 2.2 & 2.3),
//! - Asynchroniczny zrzut ramki na dysk przez `AsyncFileSink` w stylu `io_uring` (Zadanie 1.4).

use crate::pcap_sink::AsyncFileSink;
use crate::rtp_session::SansIoRtpSession;
use std::io;

pub async fn process_srtp_datagram_with_observability_and_uring<S: AsyncFileSink>(
    _raw_datagram: &[u8],
    _rx_srtp: &mut webrtc_srtp::context::Context,
    _rtp_session: &mut SansIoRtpSession,
    _pcap_sink: &mut S,
    _disk_offset: &mut u64,
    _ring_buf: Vec<u8>,
) -> io::Result<Vec<u8>> {
    // Zadanie 3.5 (Capstone): Zintegrowana pętla przetwarzania pakietu (podpowiedź: git show hint-3.5)
    todo!("Zadanie 3.5 (Capstone): Zintegruj potok pakietowy Data Plane, Control Plane i Observability")
}
