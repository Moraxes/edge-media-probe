//! Warstwa odbioru UDP (`socket2`, `MSG_PEEK` oraz pula `BytesMut`).

use std::io;
use std::net::SocketAddr;
//
use bytes::{Bytes, BytesMut};
use rtp_parser::{PacketClass, classify_peeked_packet};
use socket2::{Domain, Protocol, Socket, Type};
use tokio::net::UdpSocket;

/// Tworzy gniazdo UDP przez `socket2` z ustawionym buforem `SO_RCVBUF`,
/// przełącza je w tryb nieblokujący i konwertuje na `tokio::net::UdpSocket`.
pub fn bind_tuned_udp(
  addr: SocketAddr,
  rcv_buf_bytes: usize,
) -> io::Result<UdpSocket> {
  let domain = Domain::for_address(addr);
  let raw = Socket::new(domain, Type::DGRAM, Some(Protocol::UDP))?;
  raw.set_recv_buffer_size(rcv_buf_bytes)?;
  raw.bind(&addr.into())?;
  raw.set_nonblocking(true)?;
  UdpSocket::from_std(raw.into())
}

/// Podgląda 2-bajtowy prefiks przez `MSG_PEEK`, odzyskuje pojemność z `pool`
/// (`try_reclaim`), odbiera pełny datagram bez kopiowania oraz klasyfikuje pakiet.
pub async fn recv_rtp_or_rtcp(
  socket: &UdpSocket,
  pool: &mut BytesMut,
) -> io::Result<(Option<PacketClass>, Bytes, SocketAddr)> {
  let mut peek_buf = [0u8; 2];
  let (peek_len, _) = socket.peek_from(&mut peek_buf).await?;
  let class = classify_peeked_packet(&peek_buf[..peek_len]);

  let _ = pool.try_reclaim(2048);
  if pool.capacity() < 2048 {
    pool.reserve(2048 - pool.capacity());
  }

  let (_, peer) = socket.recv_buf_from(pool).await?;
  let frame = pool.split().freeze();
  Ok((class, frame, peer))
}

#[cfg(test)]
mod tests {
  use super::*;
  use rtp_parser::build_test_rtp_packet;

  #[tokio::test]
  async fn udp_ingress_peek_classifies_rtp_and_rtcp_with_reclaim() {
    let server =
      bind_tuned_udp("127.0.0.1:0".parse().unwrap(), 262_144).unwrap();
    let addr = server.local_addr().unwrap();
    let client = UdpSocket::bind("127.0.0.1:0")
      .await
      .unwrap();

    let rtp = build_test_rtp_packet(1, 160, 0x1122_3344, b"audio");
    client
      .send_to(&rtp, addr)
      .await
      .unwrap();

    let mut pool = BytesMut::with_capacity(4096);
    let (class1, frame1, _) = recv_rtp_or_rtcp(&server, &mut pool)
      .await
      .unwrap();
    assert_eq!(class1, Some(PacketClass::Rtp));
    assert_eq!(frame1.len(), rtp.len());
    drop(frame1);

    // Minimalny nagłówek RTCP SR (V=2, PT=200)
    let rtcp = [0x80u8, 200, 0x00, 0x06, 0x11, 0x22, 0x33, 0x44];
    client
      .send_to(&rtcp, addr)
      .await
      .unwrap();

    let (class2, frame2, _) = recv_rtp_or_rtcp(&server, &mut pool)
      .await
      .unwrap();
    assert_eq!(class2, Some(PacketClass::Rtcp));
    assert_eq!(frame2.as_ref(), &rtcp);
  }
}
