//! Niezależne narzędzie CLI `l2-dump` do zapisu, inspekcji i wstrzykiwania ramek L2 (`pcap` + `etherparse`).

use std::path::{Path, PathBuf};
//
use clap::{Parser, Subcommand};
use etherparse::{PacketBuilder, SlicedPacket, TransportSlice};
use pcap::{Activated, Active, Capture, Linktype};

#[derive(Debug, Parser)]
#[command(name = "l2-dump")]
struct Args {
  #[command(subcommand)]
  command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
  /// Buduje syntetyczną ramkę Ethernet II + IPv4 + UDP i zapisuje ją do pliku `.pcap`.
  Generate {
    path: PathBuf,
    #[arg(long, default_value = "hello-l2")]
    payload: String,
  },
  /// Odczytuje i dekoduje pakiety UDP z pliku `.pcap` lub aktywnego interfejsu.
  Inspect {
    #[arg(long)]
    file: Option<PathBuf>,
    #[arg(long)]
    live: Option<String>,
  },
  /// Wysyła syntetyczną ramkę Ethernet II + IPv4 + UDP przez wskazany interfejs.
  Inject {
    iface: String,
    #[arg(long, default_value = "hello-l2")]
    payload: String,
  },
}

/// Zadanie 13: zbuduj syntetyczną ramkę Ethernet II + IPv4 + UDP przez `PacketBuilder`.
fn build_udp_frame(payload: &[u8]) -> anyhow::Result<Vec<u8>> {
  todo!()
}

/// Zadanie 13: zapisz surową ramkę Ethernet II do pliku `.pcap` przez `Capture::dead`.
fn write_dead_pcap(path: &Path, frame: &[u8]) -> anyhow::Result<()> {
  todo!()
}

/// Zadanie 13: zdekoduj ładunki UDP z uchwytu `Capture<Activated>` dla `ETHERNET` oraz `LINUX_SLL`.
fn decode_udp_payloads<T: Activated>(
  cap: &mut Capture<T>,
  linktype: Linktype,
) -> anyhow::Result<Vec<(u16, u16, Vec<u8>)>> {
  todo!()
}

/// Zadanie 13: otwórz aktywny uchwyt na interfejsie i wyślij ramkę przez `sendpacket`.
fn inject_frame(iface: &str, frame: &[u8]) -> anyhow::Result<()> {
  todo!()
}

fn main() -> anyhow::Result<()> {
  let args = Args::parse();
  match args.command {
    Command::Generate { path, payload } => {
      let frame = build_udp_frame(payload.as_bytes())?;
      write_dead_pcap(&path, &frame)?;
      println!("Zapisano {} B do {}", frame.len(), path.display());
    }
    Command::Inspect { file, live } => match (file, live) {
      (Some(path), None) => {
        let mut cap = Capture::from_file(&path)?;
        let linktype = cap.get_datalink();
        for (src, dst, data) in decode_udp_payloads(&mut cap, linktype)? {
          println!("UDP {src} -> {dst}: {} B", data.len());
        }
      }
      (None, Some(iface)) => {
        let mut cap = Capture::from_device(iface.as_str())?
          .immediate_mode(true)
          .open()?;
        let linktype = cap.get_datalink();
        for (src, dst, data) in decode_udp_payloads(&mut cap, linktype)? {
          println!("UDP {src} -> {dst}: {} B", data.len());
        }
      }
      _ => anyhow::bail!("podaj dokładnie jedną z opcji: --file albo --live"),
    },
    Command::Inject { iface, payload } => {
      let frame = build_udp_frame(payload.as_bytes())?;
      inject_frame(&iface, &frame)?;
      println!("Wysłano {} B na {}", frame.len(), iface);
    }
  }
  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn generate_and_inspect_offline_pcap() {
    let dir =
      std::env::temp_dir().join(format!("l2-dump-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("utworzenie katalogu tymczasowego");
    let pcap_path = dir.join("test.pcap");

    let frame = build_udp_frame(b"rtp-l2-test").expect("build_udp_frame");
    write_dead_pcap(&pcap_path, &frame).expect("write_dead_pcap");

    let mut cap = Capture::from_file(&pcap_path).expect("Capture::from_file");
    let linktype = cap.get_datalink();
    let packets = decode_udp_payloads(&mut cap, linktype).expect("decode");

    assert_eq!(packets.len(), 1);
    assert_eq!(packets[0].0, 5004);
    assert_eq!(packets[0].1, 5006);
    assert_eq!(packets[0].2, b"rtp-l2-test");

    let _ = std::fs::remove_dir_all(&dir);
  }
}
