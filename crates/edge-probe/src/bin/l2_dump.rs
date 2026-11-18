//! Niezależne narzędzie CLI `l2-dump` do odkrywania interfejsów, nasłuchu zmian, sniffingu i wstrzykiwania ramek L2 (`nex` + `netwatcher` + `tokio`).

use anyhow::Context;
use rtp_parser::build_test_rtp_packet;
use serde::Serializer;
use std::{
  future::poll_fn,
  net::{IpAddr, Ipv4Addr},
};
//
use bytes::Bytes;
use clap::{Parser, Subcommand};
use futures::{StreamExt, TryStreamExt, future::ready};
use nex_core::{
  interface::{Interface, get_interfaces},
  mac::MacAddr,
};
use nex_datalink::{
  Channel, Config,
  async_io::{AsyncChannel, AsyncRawReceiver, AsyncRawSender, async_channel},
};
use nex_packet::{
  builder::{
    ethernet::EthernetPacketBuilder, ipv4::Ipv4PacketBuilder,
    udp::UdpPacketBuilder,
  },
  ethernet::{EtherType, EthernetPacket},
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
      todo!();
    }
    Command::Inspect { iface, count } => {
      let network_interface = get_interfaces()
        .into_iter()
        .find(|ifc| ifc.name == iface)
        .with_context(|| format!("interface {iface} not found"))?;
      println!("{network_interface:#?}");
      let AsyncChannel::Ethernet(_, st) = async_channel(
        &network_interface,
        Config::default().with_promiscuous(true),
      )?
      else {
        anyhow::bail!("failed to build channel");
      };

      let st = st
        .filter_map(async |maybe_frame| {
          maybe_frame
            .inspect_err(|err| eprintln!("failed to recv EthII frame: {err}"))
            .ok()
        })
        .filter_map(async |frame| {
          EthernetPacket::try_from_buf(&frame)
            .inspect_err(|err| eprintln!("failed to parse EthII frame: {err}"))
            .ok()
        })
        .inspect(|frame| {
          println!("{} -> {}", frame.source(), frame.destination())
        })
        .take(count);
      tokio::pin!(st);

      while let Some(item) = st.next().await {}
    }
    Command::Inject { iface, payload } => {
      let network_interface = get_interfaces()
        .into_iter()
        .find(|ifc| ifc.name == iface)
        .with_context(|| format!("interface {iface} not found"))?;
      println!("{network_interface:#?}");
      let AsyncChannel::Ethernet(mut tx, _) = async_channel(
        &network_interface,
        Config::default().with_promiscuous(true),
      )?
      else {
        anyhow::bail!("failed to build channel");
      };

      let localhost_addr: IpAddr = [127, 0, 0, 1].into();
      let payload_bytes = Bytes::from(payload);

      let rtp = build_test_rtp_packet(u16::MAX, u32::MAX, 0x1000, &rand::random::<[u8; 1024]>());

      let datagram = UdpPacketBuilder::new(localhost_addr, localhost_addr)
        .destination(5004)
        .source(9999)
        .payload(payload_bytes)
        .to_bytes()
        .map_err(|err| anyhow::anyhow!("failed to build UDP datagram: {err}"))?;

      let ipp = Ipv4PacketBuilder::new()
        .source(Ipv4Addr::from_octets([127, 0, 0, 1]))
        .destination(Ipv4Addr::from_octets([127, 0, 0, 1]))
        .protocol(IpNextProtocol::Udp)
        .payload(datagram)
        .to_bytes()
        .map_err(|err| anyhow::anyhow!("failed to build IP packet: {err}"))?;

      let frame = EthernetPacketBuilder::new()
        .source(network_interface.mac_addr.unwrap())
        .destination(network_interface.mac_addr.unwrap())
        .payload(ipp)
        .to_bytes();

      let frame_slice = frame.to_vec();

      println!("frame size: {}", frame.len());

      poll_fn(|cx| tx.poll_send(cx, &frame_slice)).await?;
    }
  }
  Ok(())
}

/// Zadanie 13: zbuduj syntetyczną ramkę Ethernet II + IPv4 + UDP przez `nex-packet`.
fn build_udp_frame(payload: &[u8]) -> anyhow::Result<Vec<u8>> {
  let _ = payload;
  todo!()
}

/// Zadanie 13: zdekoduj ładunki UDP ze strumienia `AsyncRawReceiver` przez `FrameSlice`.
async fn decode_udp_payloads(
  rx: &mut dyn AsyncRawReceiver,
  limit: usize,
) -> anyhow::Result<Vec<(u16, u16, Vec<u8>)>> {
  let _ = (rx, limit);
  todo!()
}

/// Zadanie 13: wyślij surową ramkę asynchronicznie przez `AsyncRawSender::poll_send`.
async fn inject_frame(
  tx: &mut dyn AsyncRawSender,
  frame: &[u8],
) -> anyhow::Result<()> {
  let _ = (tx, frame);
  todo!()
}
