//! Symulator ruchu RTP nadawanego przez UDP do testowania sondy `edge-probe`.

use std::net::SocketAddr;
use std::time::Duration;
//
use clap::Parser;
use rtp_parser::build_test_rtp_packet;
use tokio::net::UdpSocket;
use tokio::signal;
use tokio::time::interval;

fn parse_ssrc_arg(raw: &str) -> Result<u32, String> {
  edge_probe::parse_ssrc(raw)
    .ok_or_else(|| format!("nieprawidłowe SSRC: {raw}"))
}

#[derive(Debug, Parser)]
struct Args {
  #[arg(long, default_value = "127.0.0.1:5004")]
  target: SocketAddr,

  #[arg(long = "ssrc", value_parser = parse_ssrc_arg)]
  ssrcs: Vec<u32>,

  #[arg(long, default_value_t = 40)]
  interval_ms: u64,

  #[arg(long, default_value_t = 50)]
  drop_every: u16,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let args = Args::parse();
  let ssrcs = if args.ssrcs.is_empty() {
    vec![0x1000]
  } else {
    args.ssrcs
  };

  let socket = UdpSocket::bind("127.0.0.1:0").await?;
  let ssrc_list: Vec<String> = ssrcs
    .iter()
    .map(|s| format!("{s:#010x}"))
    .collect();
  println!(
    "udp-sim: wysyłam datagramy RTP z {} na {} (SSRC: {}, interwał: {} ms, drop_every: {})",
    socket.local_addr()?,
    args.target,
    ssrc_list.join(", "),
    args.interval_ms,
    args.drop_every
  );

  let mut seqs = vec![0u16; ssrcs.len()];
  let mut timestamps = vec![0u32; ssrcs.len()];
  let mut ticker = interval(Duration::from_millis(args.interval_ms));

  loop {
    tokio::select! {
        _ = signal::ctrl_c() => {
            println!("udp-sim: zatrzymano");
            break;
        }
        _ = ticker.tick() => {
            for (idx, &ssrc) in ssrcs.iter().enumerate() {
                seqs[idx] = seqs[idx].wrapping_add(1);
                if args.drop_every > 0 && seqs[idx].is_multiple_of(args.drop_every) {
                    seqs[idx] = seqs[idx].wrapping_add(1);
                }
                timestamps[idx] = timestamps[idx].wrapping_add(3000);
                let frame = build_test_rtp_packet(
                    seqs[idx],
                    timestamps[idx],
                    ssrc,
                    b"rtp-test-payload",
                );
                socket.send_to(&frame, args.target).await?;
            }
        }
    }
  }

  Ok(())
}
