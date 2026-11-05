use bytes::BytesMut;
use clap::Parser;
use edge_media_probe::observability::{
    init_tracing, spawn_runtime_metrics_collector, ProbeDataplaneMetrics,
};
use edge_media_probe::pcap_sink::PortableUringStyleSink;
use edge_media_probe::pipeline::process_srtp_datagram_with_observability_and_uring;
use edge_media_probe::rtp_session::SansIoRtpSession;
use edge_media_probe::srtp_layer::create_srtp_pair;
use edge_media_probe::udp_ingress::bind_tuned_udp;
use metrics_exporter_prometheus::PrometheusBuilder;
use std::net::SocketAddr;
use std::time::{Duration, Instant};

/// Główny węzeł sieciowy Edge Media Probe (sonda i przekaźnik RTP/SRTP)
#[derive(Parser, Debug)]
#[command(name = "edge_probe", version, about = "Główny węzeł sieciowy Edge Media Probe (sonda i przekaźnik RTP/SRTP)")]
struct Args {
    /// Adres nasłuchu UDP dla strumienia RTP/SRTP
    #[arg(short, long, default_value = "127.0.0.1:5004")]
    listen: SocketAddr,

    /// Opcjonalny adres docelowy UDP do przekazywania odszyfrowanych pakietów RTP
    #[arg(short, long)]
    forward: Option<SocketAddr>,

    /// Adres HTTP dla eksportera metryk Prometheusa
    #[arg(short, long, default_value = "0.0.0.0:3002")]
    metrics: SocketAddr,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // Inicjalizacja nieblokującego potoku telemetrii (tracing + opcjonalnie Tokio Console)
    let _guard = init_tracing("edge_probe");

    // Inicjalizacja eksportera metryk Prometheusa dla warstwy Data Plane (port 3002)
    // Buckety w mikrosekundach (0.5ms .. 100ms) wymuszają eksport jako Histogram z _bucket, zamiast Summary
    let buckets = &[
        500.0, 1000.0, 2000.0, 5000.0, 10_000.0, 20_000.0, 30_000.0, 40_000.0, 50_000.0, 100_000.0,
    ];
    let builder_res = PrometheusBuilder::new()
        .set_buckets(buckets)
        .and_then(|b| b.with_http_listener(args.metrics).install());

    if let Err(e) = builder_res {
        tracing::warn!(%e, "Nie udało się uruchomić serwera metryk Prometheusa na {}", args.metrics);
    }

    // Inicjalizacja liczników w Prometheusie na 0, aby panele nie zgłaszały braku danych ("No data")
    metrics::counter!("probe_rtp_packets_lost_total").increment(0);
    metrics::counter!("probe_srtp_auth_failures_total").increment(0);
    metrics::gauge!("probe_rtp_loss_ratio").set(0.0);

    // Kolektor metryk egzekutora Tokio dla sondy
    spawn_runtime_metrics_collector();

    let listen_addr = args.listen;
    let forward_addr = args.forward;
    let metrics_addr = args.metrics;

    println!("⚡ [Edge Media Probe] Start węzła sieciowego:");
    println!("   • Nasłuch UDP:    {listen_addr}");
    println!("   • Metryki HTTP:   http://{metrics_addr}/metrics");
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
    let mut last_packet_instant: Option<Instant> = None;

    loop {
        pool.clear();
        let (n, _src) = rx_socket.recv_buf_from(&mut pool).await?;
        let datagram = pool.split_to(n).freeze();
        let now = Instant::now();
        let jitter_us = last_packet_instant
            .map(|prev| now.duration_since(prev).as_micros() as f64)
            .unwrap_or(0.0);
        last_packet_instant = Some(now);

        ring_buf = process_srtp_datagram_with_observability_and_uring(
            &datagram,
            &mut rx_srtp,
            &mut rtp_session,
            &mut pcap_sink,
            &mut disk_offset,
            ring_buf,
        )
        .await?;

        // Okresowe sprawdzanie raportów RTCP Receiver Report (RR)
        rtp_session.handle_timeout(now);
        while let Some(output) = rtp_session.poll_output() {
            if let edge_media_probe::rtp_session::ProtocolOutput::SendRtcpReceiverReport {
                lost_packets,
                extended_highest_seq,
                ..
            } = output
            {
                ProbeDataplaneMetrics::on_rtcp_receiver_report(lost_packets, extended_highest_seq);
            }
        }

        // Jeśli skonfigurowano port docelowy (np. 5006 dla VLC), przekazujemy surowy pakiet dalej:
        if let Some(fwd) = forward_addr {
            let _ = tx_socket.send_to(&datagram, fwd).await;
        }

        ProbeDataplaneMetrics::on_packet("raw_udp", n, jitter_us);
    }
}

