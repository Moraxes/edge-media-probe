use std::net::UdpSocket;

fn main() -> anyhow::Result<()> {
    let socket = UdpSocket::bind("127.0.0.1:5006")?;
    println!("📺 [Receiver] Oczekiwanie na przekazany strumień z sondy na 127.0.0.1:5006...");

    let mut buf = [0u8; 1500];
    let mut expected_seq: Option<u16> = None;

    loop {
        let (n, _) = socket.recv_from(&mut buf)?;
        if n >= 12 && (buf[0] >> 6) == 2 {
            let seq = u16::from_be_bytes([buf[2], buf[3]]);
            if let Some(exp) = expected_seq {
                if seq != exp {
                    println!("\x1b[31m[Receiver] LUKA W SEKWENCJI: Oczekiwano #{exp}, otrzymano #{seq}!\x1b[0m");
                } else {
                    println!("\x1b[32m[Receiver] OK: Ramka #{seq} odebrana poprawnie ({n} B)\x1b[0m");
                }
            }
            expected_seq = Some(seq.wrapping_add(1));
        }
    }
}

