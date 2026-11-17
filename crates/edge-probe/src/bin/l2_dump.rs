//! Niezależne narzędzie CLI `l2-dump` do odkrywania interfejsów, nasłuchu zmian, sniffingu i wstrzykiwania ramek L2 (`nex` + `netwatcher` + `tokio`).

use std::future::poll_fn;
//
use bytes::Bytes;
use clap::{Parser, Subcommand};
use futures::StreamExt;
use nex_core::{
  interface::{Interface, get_interfaces},
  mac::MacAddr,
};
use nex_datalink::{
  Config,
  async_io::{AsyncChannel, AsyncRawReceiver, AsyncRawSender, async_channel},
};
use nex_packet::{
  builder::{
    ethernet::EthernetPacketBuilder, ipv4::Ipv4PacketBuilder,
    udp::UdpPacketBuilder,
  },
  ethernet::EtherType,
  frame::{FrameSlice, ParseOption},
  ip::IpNextProtocol,
  packet::Packet,
};

#[derive(Debug, Parser)]
#[command(name = "l2-dump")]
struct Args {
  #[command(subcommand)]
  command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
  /// Listuje dostępne interfejsy sieciowe hosta.
  Devices,
  /// Nasłuchuje zdarzeń dodania/usunięcia interfejsów oraz zmian adresów IP (`netwatcher`).
  Listen,
  /// Odbiera i dekoduje pakiety UDP na wskazanym interfejsie.
  Inspect {
    iface: String,
    #[arg(long, default_value_t = 10)]
    count: usize,
  },
  /// Wysyła syntetyczną ramkę Ethernet II + IPv4 + UDP przez wskazany interfejs.
  Inject {
    iface: String,
    #[arg(long, default_value = "hello-l2")]
    payload: String,
  },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  let args = Args::parse();
  match args.command {
    Command::Devices => {
      for iface in get_interfaces() {
        println!("{} (mac: {:?})", iface.name, iface.mac_addr);
      }
    }
    Command::Listen => {
      let mut watch = netwatcher::watch_interfaces_async::<
        netwatcher::async_adapter::Tokio,
      >()?;
      loop {
        let update = watch.changed().await;
        for iface in update.diff.added.values() {
          println!("+ iface {} (index {})", iface.name, iface.index);
        }
        for iface in update.diff.removed.values() {
          println!("- iface {} (index {})", iface.name, iface.index);
        }
        for (ifindex, addr) in update.addrs_added() {
          println!("+ addr ifindex={ifindex}: {}/{}", addr.ip, addr.prefix_len);
        }
        for (ifindex, addr) in update.addrs_removed() {
          println!("- addr ifindex={ifindex}: {}/{}", addr.ip, addr.prefix_len);
        }
      }
    }
    Command::Inspect { iface, count } => {
      let dev = find_interface(&iface)?;
      let AsyncChannel::Ethernet(_tx, mut rx) =
        async_channel(&dev, Config::default())?
      else {
        anyhow::bail!("nieobsługiwany typ kanału L2");
      };
      for (src, dst, data) in decode_udp_payloads(rx.as_mut(), count).await? {
        println!("UDP {src} -> {dst}: {} B", data.len());
      }
    }
    Command::Inject { iface, payload } => {
      let dev = find_interface(&iface)?;
      let AsyncChannel::Ethernet(mut tx, _rx) =
        async_channel(&dev, Config::default())?
      else {
        anyhow::bail!("nieobsługiwany typ kanału L2");
      };
      let frame = build_udp_frame(payload.as_bytes())?;
      inject_frame(tx.as_mut(), &frame).await?;
      println!("Wysłano {} B na {}", frame.len(), iface);
    }
  }
  Ok(())
}

fn find_interface(name: &str) -> anyhow::Result<Interface> {
  get_interfaces()
    .into_iter()
    .find(|iface| iface.name == name)
    .ok_or_else(|| anyhow::anyhow!("nie znaleziono interfejsu: {name}"))
}

/// Zadanie 13: zbuduj syntetyczną ramkę Ethernet II + IPv4 + UDP przez `nex-packet`.
fn build_udp_frame(payload: &[u8]) -> anyhow::Result<Vec<u8>> {
  let src_ip = std::net::Ipv4Addr::new(10, 0, 0, 1);
  let dst_ip = std::net::Ipv4Addr::new(10, 0, 0, 2);

  let udp = UdpPacketBuilder::new(src_ip.into(), dst_ip.into())
    .source(5004)
    .destination(5006)
    .payload(Bytes::copy_from_slice(payload))
    .calculate_checksum()
    .build()?;

  let ipv4 = Ipv4PacketBuilder::new()
    .source(src_ip)
    .destination(dst_ip)
    .ttl(64)
    .protocol(IpNextProtocol::Udp)
    .payload(udp.to_bytes())
    .build()?;

  let eth = EthernetPacketBuilder::new()
    .source(MacAddr::new(1, 2, 3, 4, 5, 6))
    .destination(MacAddr::new(7, 8, 9, 10, 11, 12))
    .ethertype(EtherType::Ipv4)
    .payload(ipv4.to_bytes())
    .build();

  Ok(eth.to_bytes().to_vec())
}

/// Zadanie 13: zdekoduj ładunki UDP ze strumienia `AsyncRawReceiver` przez `FrameSlice`.
async fn decode_udp_payloads(
  rx: &mut dyn AsyncRawReceiver,
  limit: usize,
) -> anyhow::Result<Vec<(u16, u16, Vec<u8>)>> {
  let mut out = Vec::new();
  while out.len() < limit {
    let Some(item) = rx.next().await else {
      break;
    };
    let raw = item?;
    let Ok(sliced) = FrameSlice::try_from_buf(&raw, ParseOption::default())
    else {
      continue;
    };
    if sliced.ip_protocol != Some(IpNextProtocol::Udp) {
      continue;
    }
    let Some(transport) = sliced.transport else {
      continue;
    };
    if transport.len() < 4 {
      continue;
    }
    let src = u16::from_be_bytes([transport[0], transport[1]]);
    let dst = u16::from_be_bytes([transport[2], transport[3]]);
    out.push((src, dst, sliced.payload.to_vec()));
  }
  Ok(out)
}

/// Zadanie 13: wyślij surową ramkę asynchronicznie przez `AsyncRawSender::poll_send`.
async fn inject_frame(
  tx: &mut dyn AsyncRawSender,
  frame: &[u8],
) -> anyhow::Result<()> {
  poll_fn(|cx| tx.poll_send(cx, frame)).await?;
  Ok(())
}

#[cfg(test)]
mod tests {
  use tokio::sync::mpsc;
  use tokio_stream::wrappers::ReceiverStream;

  use super::*;

  #[tokio::test]
  async fn build_and_decode_udp_frame_over_channel() {
    let frame = build_udp_frame(b"rtp-l2-test").expect("build_udp_frame");
    let (tx, rx) = mpsc::channel(4);
    tx.send(Ok(frame))
      .await
      .expect("send frame");
    drop(tx);

    let mut stream = ReceiverStream::new(rx);
    let packets = decode_udp_payloads(&mut stream, 1)
      .await
      .expect("decode_udp_payloads");

    assert_eq!(packets.len(), 1);
    assert_eq!(packets[0].0, 5004);
    assert_eq!(packets[0].1, 5006);
    assert_eq!(packets[0].2, b"rtp-l2-test");
  }
}
