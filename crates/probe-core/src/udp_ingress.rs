//! Moduł 3 (Zadanie 3.1): Demultipleksacja UDP (`rtcp-mux`) i strojenie gniazda.
//!
//! Ćwiczenie obejmuje:
//! - Strojenie gniazda w `socket2` (`SO_RCVBUF`, tryb nieblokujący),
//! - Recykling pamięci w czasie $O(1)$ przez `BytesMut::try_reclaim`.
//!
//! Parsowanie nagłówków RTP (RFC 3550) i klasyfikacja `rtcp-mux` (RFC 5761)
//! są dostarczane jako gotowe komponenty przez kratkę pomocniczą `rtp-parser`.

use socket2::{Domain, Protocol, Socket, Type};
use std::net::SocketAddr;
use tokio::net::UdpSocket;

// Re-eksport gotowych narzędzi parsowania z kratki pomocniczej rtp-parser
pub use rtp_parser::{
    build_test_rtp_packet, classify_peeked_packet, parse_rtp_header, PacketClass, RtpHeader,
    RtpParseError,
};

pub fn bind_tuned_udp(_addr: SocketAddr, _rcv_buf_bytes: usize) -> std::io::Result<UdpSocket> {
    // Zadanie 3.1: Skonfiguruj gniazdo UDP z socket2 i przekaż do Tokio (podpowiedź: git show hint-3.1)
    todo!("Zadanie 3.1: Zaimplementuj bind_tuned_udp z socket2")
}
