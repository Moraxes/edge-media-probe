use clap::Parser;
use probe_core::srtp_layer::create_srtp_pair;
use rtp::header::Header;
use rtp::packet::Packet;
use std::net::{SocketAddr, UdpSocket};
use std::time::Duration;
use webrtc_util::marshal::Marshal;

/// Syntetyczny generator strumienia RTP/SRTP (pure-Rust)
#[derive(Parser, Debug)]
#[command(name = "mock_media_sender", version, about = "Syntetyczny generator strumienia RTP/SRTP")]
struct Args {
    /// Adres docelowy strumienia RTP/SRTP (np. adres sondy)
    #[arg(short, long, default_value = "127.0.0.1:5004")]
    target: SocketAddr,

    /// Adres lokalnego gniazda do zbindowania
    #[arg(short, long, default_value = "127.0.0.1:0")]
    bind: SocketAddr,

    /// Liczba klatek/pakietów na sekundę (fps/pps)
    #[arg(long, default_value_t = 30)]
    fps: u64,

    /// Częstotliwość celowego gubienia pakietów do testu NACK (co N-ty pakiet, 0 wyłącza gubienie)
    #[arg(long, default_value_t = 50)]
    drop_every: u16,

    /// Szyfruje ładunek i dokleja 10-bajtowy tag uwierzytelnienia SRTP (RFC 3711)
    #[arg(long)]
    srtp: bool,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let socket = UdpSocket::bind(args.bind)?;
    let target = args.target;
    let fps = args.fps.max(1);
    let sleep_duration = Duration::from_millis(1000 / fps);

    let mut tx_srtp = if args.srtp {
        let (tx_ctx, _) = create_srtp_pair()?;
        Some(tx_ctx)
    } else {
        None
    };

    println!("[Sender] Wysyłanie syntetycznego strumienia na {target} ({fps} pps)...");
    println!("   - Format pakietów: webrtc-rs/rtp (RFC 3550 Packet::marshal)");
    println!("   - Tryb SRTP:       {}", if args.srtp { "aktywny (--srtp)" } else { "wyłączony (jawny RTP)" });

    let mut seq: u16 = 1;
    let mut ts: u32 = 90_000;

    loop {
        // Celowo gubimy pakiety o numerze podzielnym przez drop_every, aby przetestować NACK w sondzie
        if args.drop_every == 0 || !seq.is_multiple_of(args.drop_every) {
            // W zastosowaniach produkcyjnych WebRTC pakiety budowane są przez kratkę `rtp`:
            let rtp_packet = Packet {
                header: Header {
                    version: 2,
                    padding: false,
                    extension: false,
                    marker: seq.is_multiple_of(30),
                    payload_type: 96,
                    sequence_number: seq,
                    timestamp: ts,
                    ssrc: 0xCAFE_BABE,
                    ..Default::default()
                },
                payload: bytes::Bytes::from_static(&[0xAA; 160]),
            };

            let raw_packet = rtp_packet.marshal()?;
            let wire_packet = if let Some(srtp) = &mut tx_srtp {
                srtp.encrypt_rtp(&raw_packet)?
            } else {
                raw_packet
            };
            socket.send_to(&wire_packet, target)?;
        } else {
            eprintln!("[Sender] Celowo zgubiono pakiet seq={seq} (test NACK)");
        }

        seq = seq.wrapping_add(1);
        ts = ts.wrapping_add(3000);
        std::thread::sleep(sleep_duration);
    }
}
