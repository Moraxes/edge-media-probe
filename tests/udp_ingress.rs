use bytes::{Bytes, BytesMut};
use edge_media_probe::udp_ingress::{
    bind_tuned_udp, build_test_rtp_packet, classify_peeked_packet, parse_rtp_header, PacketClass,
};
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
    assert!(pool.capacity() >= 4096 - n);
    Ok(())
}

