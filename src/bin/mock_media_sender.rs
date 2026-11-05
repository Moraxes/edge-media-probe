use rtp::header::Header;
use rtp::packet::Packet;
use std::net::UdpSocket;
use std::time::Duration;
use webrtc_util::marshal::Marshal;

fn main() -> anyhow::Result<()> {
    let socket = UdpSocket::bind("127.0.0.1:0")?;
    let target = "127.0.0.1:5004";
    println!("🎥 [Sender] Wysyłanie syntetycznego strumienia RTP na {target} (30 pps)...");
    println!("   • Format pakietów: webrtc-rs/rtp (RFC 3550 Packet::marshal)");

    let mut seq: u16 = 1;
    let mut ts: u32 = 90_000;

    loop {
        // Celowo gubimy pakiety o numerze podzielnym przez 50, aby przetestować NACK w sondzie!
        if seq % 50 != 0 {
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
        std::thread::sleep(Duration::from_millis(33)); // ~30 fps
    }
}
