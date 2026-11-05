use clap::Parser;
use rtp::header::Header;
use rtp::packet::Packet;
use std::net::{SocketAddr, UdpSocket};
use std::time::Duration;
use webrtc_util::marshal::Marshal;

/// Syntetyczny generator strumienia RTP (pure-Rust)
#[derive(Parser, Debug)]
#[command(name = "mock_media_sender", version, about = "Syntetyczny generator strumienia RTP")]
struct Args {
    /// Adres docelowy strumienia RTP (np. adres sondy)
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
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let socket = UdpSocket::bind(args.bind)?;
    let target = args.target;
    let fps = args.fps.max(1);
    let sleep_duration = Duration::from_millis(1000 / fps);

    println!("🎥 [Sender] Wysyłanie syntetycznego strumienia RTP na {target} ({fps} pps)...");
    println!("   • Format pakietów: webrtc-rs/rtp (RFC 3550 Packet::marshal)");

    let mut seq: u16 = 1;
    let mut ts: u32 = 90_000;

    loop {
        // Celowo gubimy pakiety o numerze podzielnym przez drop_every, aby przetestować NACK w sondzie!
        if args.drop_every == 0 || seq % args.drop_every != 0 {
            // W zastosowaniach produkcyjnych WebRTC pakiety budowane są przez kratkę `rtp`:
            let rtp_packet = Packet {
                header: Header {
                    version: 2,
                    padding: false,
                    extension: false,
                    marker: (seq % 30 == 0),
                    payload_type: 96,
                    sequence_number: seq,
                    timestamp: ts,
                    ssrc: 0xCAFE_BABE,
                    ..Default::default()
                },
                payload: bytes::Bytes::from_static(&[0xAA; 160]),
            };

            let raw_packet = rtp_packet.marshal()?;
            socket.send_to(&raw_packet, target)?;
        } else {
            eprintln!("💥 [Sender] Celowo zgubiono pakiet seq={seq} (test NACK)!");
        }

        seq = seq.wrapping_add(1);
        ts = ts.wrapping_add(3000);
        std::thread::sleep(sleep_duration);
    }
}
