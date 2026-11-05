use std::net::UdpSocket;
use std::time::Duration;

fn main() -> anyhow::Result<()> {
    let socket = UdpSocket::bind("127.0.0.1:0")?;
    let target = "127.0.0.1:5004";
    println!("🎥 [Sender] Wysyłanie syntetycznego strumienia RTP na {target} (30 pps)...");

    let mut seq: u16 = 1;
    let mut ts: u32 = 90_000;

    loop {
        // Celowo gubimy pakiety o numerze podzielnym przez 50, aby przetestować NACK w sondzie!
        if seq % 50 != 0 {
            let mut packet = Vec::with_capacity(172);
            packet.push(0x80); // V=2, P=0, X=0, CC=0
            packet.push(96);   // PT=96 (H.264)
            packet.extend_from_slice(&seq.to_be_bytes());
            packet.extend_from_slice(&ts.to_be_bytes());
            packet.extend_from_slice(&0xCAFE_BABEu32.to_be_bytes()); // SSRC
            packet.extend_from_slice(&[0xAA; 160]);                  // Ładunek
            socket.send_to(&packet, target)?;
        } else {
            eprintln!("💥 [Sender] Celowo zgubiono pakiet seq={seq} (test NACK)!");
        }

        seq = seq.wrapping_add(1);
        ts = ts.wrapping_add(3000);
        std::thread::sleep(Duration::from_millis(33)); // ~30 fps
    }
}

