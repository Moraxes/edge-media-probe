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

// Re-eksport gotowych narzędzi parsowania z kratki rtp-parser
pub use rtp_parser::{
    build_test_rtp_packet, classify_peeked_packet, parse_rtp_header, PacketClass, RtpHeader,
    RtpParseError,
};

pub fn bind_tuned_udp(addr: SocketAddr, rcv_buf_bytes: usize) -> std::io::Result<UdpSocket> {
    let domain = Domain::for_address(addr);
    let socket = Socket::new(domain, Type::DGRAM, Some(Protocol::UDP))?;
    socket.set_reuse_address(true)?;
    let _ = socket.set_recv_buffer_size(rcv_buf_bytes);
    socket.set_nonblocking(true)?;
    socket.bind(&addr.into())?;
    UdpSocket::from_std(socket.into())
}
