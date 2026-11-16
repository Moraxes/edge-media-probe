//! Warstwa odbioru UDP (`socket2`, `MSG_PEEK` oraz pula `BytesMut`).

use std::io;
use std::net::SocketAddr;
//
use bytes::{Bytes, BytesMut};
use rtp_parser::{classify_peeked_packet, PacketClass};
use socket2::{Domain, Protocol, Socket, Type};
use tokio::net::UdpSocket;

/// Zadanie 8: utwórz gniazdo UDP przez `socket2`, ustaw `SO_RCVBUF` oraz tryb
/// nieblokujący przed `bind()`, a następnie przekonwertuj do
/// `tokio::net::UdpSocket`.
pub fn bind_tuned_udp(
  _addr: SocketAddr,
  _rcv_buf_bytes: usize,
) -> io::Result<UdpSocket> {
  todo!()
}

/// Zadanie 12: utwórz gniazdo UDP z `SO_REUSEPORT` (`#[cfg(unix)]`),
/// `SO_RCVBUF` oraz trybem nieblokującym przed `bind()`, aby wiele workerów
/// mogło odbierać datagramy równolegle na tym samym porcie.
pub fn bind_reuseport_udp(
  _addr: SocketAddr,
  _rcv_buf_bytes: usize,
) -> io::Result<UdpSocket> {
  todo!()
}

/// Zadanie 8: odzyskaj pojemność puli (`try_reclaim(1500)` / `reserve(1500)`),
/// podejrzyj pierwsze 2 bajty przez `socket.peek_from(&mut [0u8; 2]).await`, sklasyfikuj
/// datagram (`classify_peeked_packet`), odbierz go przez `recv_buf_from` i wydziel `Bytes`.
pub async fn recv_rtp_or_rtcp(
  _socket: &UdpSocket,
  _pool: &mut BytesMut,
) -> io::Result<(Option<PacketClass>, Bytes, SocketAddr)> {
  todo!()
}
