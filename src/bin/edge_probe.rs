use bytes::BytesMut;
use edge_media_probe::observability::ProbeDataplaneMetrics;
use edge_media_probe::pcap_sink::PortableUringStyleSink;
use edge_media_probe::pipeline::process_srtp_datagram_with_observability_and_uring;
use edge_media_probe::rtp_session::SansIoRtpSession;
use edge_media_probe::srtp_layer::create_srtp_pair;
use edge_media_probe::udp_ingress::bind_tuned_udp;
use std::env;
use std::net::SocketAddr;
use std::time::{Duration, Instant};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().collect();
    let listen_addr: SocketAddr = args
        .iter()
        .position(|a| a == "--listen")
        .and_then(|i| args.get(i + 1))
        .unwrap_or(&"127.0.0.1:5004".to_string())
        .parse()?;

    let forward_addr: Option<SocketAddr> = args
        .iter()
        .position(|a| a == "--forward")
        .and_then(|i| args.get(i + 1))
        .and_then(|s| s.parse().ok());

    println!("⚡ [Edge Media Probe] Start węzła sieciowego:");
    println!("   • Nasłuch UDP:    {listen_addr}");
    if let Some(fwd) = forward_addr {
        println!("   • Przekazywanie:  {fwd} (np. VLC / ffplay stream.sdp)");
    } else {
        println!("   • Przekazywanie:  brak (tryb czystej sondy / sniffera)");
    }

    let rx_socket = bind_tuned_udp(listen_addr, 512 * 1024)?;
    let tx_socket = tokio::net::UdpSocket::bind("127.0.0.1:0").await?;

    let (_, mut rx_srtp) = create_srtp_pair()?;
    let mut rtp_session = SansIoRtpSession::new(0xCAFE_BABE, Instant::now(), Duration::from_secs(1));

    let dump_path = std::env::temp_dir().join("probe_emergency_dump.bin");
    let mut pcap_sink = PortableUringStyleSink::create(&dump_path).await?;
    let mut disk_offset = 0u64;
    let mut ring_buf = Vec::with_capacity(2048);

    let mut pool = BytesMut::with_capacity(4096);

    loop {
        pool.clear();
        let (n, _src) = rx_socket.recv_buf_from(&mut pool).await?;
        let datagram = pool.split_to(n).freeze();

        ring_buf = process_srtp_datagram_with_observability_and_uring(
            &datagram,
            &mut rx_srtp,
            &mut rtp_session,
            &mut pcap_sink,
            &mut disk_offset,
            ring_buf,
        )
        .await?;

        // Jeśli skonfigurowano port docelowy (np. 5006 dla VLC), przekazujemy surowy pakiet dalej:
        if let Some(fwd) = forward_addr {
            let _ = tx_socket.send_to(&datagram, fwd).await;
        }

        ProbeDataplaneMetrics::on_packet("raw_udp", n, 0.0);
    }
}

