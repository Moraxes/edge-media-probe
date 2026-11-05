//! Moduł 3 (Zadanie 3.1): Demultipleksacja UDP (`rtcp-mux`) i pula buforów $O(1)$.
//!
//! Ćwiczenie obejmuje:
//! - Strojenie gniazda w `socket2` (`SO_RCVBUF`, tryb nieblokujący),
//! - Bezinwazyjny podgląd `SockRef::peek_from` (`MSG_PEEK`) rozróżniający RTP od RTCP (PT 200..=208),
//! - Parser nagłówka RTP oparty na kursorach `try_get_*` z `bytes::Buf`,
//! - Recykling pamięci w czasie $O(1)$ przez `BytesMut::try_reclaim`.

use bytes::{Buf, BufMut, Bytes, BytesMut};
use socket2::{Domain, Protocol, Socket, Type};
use std::net::SocketAddr;
use tokio::net::UdpSocket;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PacketClass {
    Rtp,
    Rtcp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RtpHeader {
    pub version: u8,
    pub marker: bool,
    pub payload_type: u8,
    pub sequence_number: u16,
    pub timestamp: u32,
    pub ssrc: u32,
}

pub fn bind_tuned_udp(addr: SocketAddr, rcv_buf_bytes: usize) -> std::io::Result<UdpSocket> {
    let domain = Domain::for_address(addr);
    let socket = Socket::new(domain, Type::DGRAM, Some(Protocol::UDP))?;
    socket.set_reuse_address(true)?;
    let _ = socket.set_recv_buffer_size(rcv_buf_bytes);
    socket.set_nonblocking(true)?;
    socket.bind(&addr.into())?;
    Ok(UdpSocket::from_std(socket.into())?)
}

pub fn classify_peeked_packet(peek_buf: &[u8]) -> Option<PacketClass> {
    if peek_buf.len() < 2 {
        return None;
    }
    let version = peek_buf[0] >> 6;
    if version != 2 {
        return None;
    }
    // RFC 5761 (rtcp-mux): typy ładunku RTCP (SR=200, RR=201, SDES=202, BYE=203, APP=204, RTPFB=205, PSFB=206)
    if (200..=208).contains(&peek_buf[1]) {
        Some(PacketClass::Rtcp)
    } else {
        Some(PacketClass::Rtp)
    }
}

pub fn parse_rtp_header(mut packet: Bytes) -> anyhow::Result<(RtpHeader, Bytes)> {
    let b0 = packet.try_get_u8()?;
    let b1 = packet.try_get_u8()?;
    let version = b0 >> 6;
    anyhow::ensure!(version == 2, "Nieprawidłowa wersja RTP: {version}");
    let csrc_count = (b0 & 0x0F) as usize;
    let marker = (b1 & 0x80) != 0;
    let payload_type = b1 & 0x7F;
    let sequence_number = packet.try_get_u16()?;
    let timestamp = packet.try_get_u32()?;
    let ssrc = packet.try_get_u32()?;

    let csrc_bytes = csrc_count * 4;
    anyhow::ensure!(packet.remaining() >= csrc_bytes, "Ucięta lista CSRC");
    packet.advance(csrc_bytes);

    Ok((
        RtpHeader {
            version,
            marker,
            payload_type,
            sequence_number,
            timestamp,
            ssrc,
        },
        packet,
    ))
}

pub fn build_test_rtp_packet(seq: u16, ts: u32, ssrc: u32, payload: &[u8]) -> Bytes {
    let mut buf = BytesMut::with_capacity(12 + payload.len());
    buf.put_u8(0x80); // V=2, P=0, X=0, CC=0
    buf.put_u8(96);   // M=0, PT=96 (dynamiczny H.264/Opus)
    buf.put_u16(seq);
    buf.put_u32(ts);
    buf.put_u32(ssrc);
    buf.put_slice(payload);
    buf.freeze()
}

#[cfg(test)]
mod tests {
    use super::*;
    use socket2::SockRef;
    use std::mem::MaybeUninit;

    #[tokio::test]
    async fn test_udp_peek_demux_and_zero_copy_reclaim() -> anyhow::Result<()> {
        let rx = bind_tuned_udp("127.0.0.1:0".parse()?, 256 * 1024)?;
        let tx = bind_tuned_udp("127.0.0.1:0".parse()?, 256 * 1024)?;
        let rx_addr = rx.local_addr()?;

        let pkt = build_test_rtp_packet(42, 160_000, 0xCAFE_BABE, &[0xAA; 160]);
        tx.send_to(&pkt, rx_addr).await?;

        rx.readable().await?;
        let sock_ref = SockRef::from(&rx);
        let mut peek_raw = [MaybeUninit::new(0u8); 2];
        let (peek_len, _) = sock_ref.peek_from(&mut peek_raw)?;
        let peek_slice = unsafe { std::slice::from_raw_parts(peek_raw.as_ptr().cast::<u8>(), peek_len) };
        assert_eq!(classify_peeked_packet(peek_slice), Some(PacketClass::Rtp));

        let mut pool = BytesMut::with_capacity(4096);
        let initial_ptr = pool.as_ptr();

        let (n, _) = rx.recv_buf_from(&mut pool).await?;
        let raw_datagram: Bytes = pool.split_to(n).freeze();
        let (hdr, payload) = parse_rtp_header(raw_datagram)?;

        assert_eq!(hdr.sequence_number, 42);
        assert_eq!(payload.len(), 160);

        drop(payload);
        let rec = pool.try_reclaim(160);
        println!("initial: {:?}, current: {:?}, reclaimed: {}, cap: {}", initial_ptr, pool.as_ptr(), rec, pool.capacity());
        assert!(rec);
        // Po zwolnieniu i ponownym zarezerwowaniu lub ponownym wyczyszczeniu:
        assert!(pool.capacity() >= 4096 - n);
        Ok(())
    }
}

