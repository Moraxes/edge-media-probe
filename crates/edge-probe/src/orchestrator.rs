//! Nadzór nad zadaniami strumieni i ich bezpieczne wygaszanie (*graceful shutdown*).

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
//
use bytes::Bytes;
use chrono::Utc;
use rtp_parser::parse_rtp_header;
use tokio::net::UdpSocket;
use tokio::signal;
use tokio::sync::broadcast::error::RecvError;
use tokio::sync::{broadcast, mpsc};
use tokio::time::{sleep, timeout};
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;
//
use crate::control_client::ControlClient;
use crate::control_plane::{SlaAlertPayload, StreamInfo};
use crate::kms_client::{self, KmsKeyResponse};

/// Zadanie 3: uruchom w `TaskTracker` zadania dla kluczy z Zadania 2
/// i przeprowadź *graceful shutdown* po otrzymaniu `Ctrl+C`.
pub async fn supervise_keys(keys: HashMap<u32, KmsKeyResponse>) {
  let tracker = TaskTracker::new();
  let token = CancellationToken::new();

  for (ssrc, key) in keys {
    let tok = token.clone();
    tracker.spawn(async move {
      let ttl = (key.expires_at - Utc::now())
        .to_std()
        .unwrap_or(Duration::ZERO);
      println!("Start nadzoru SSRC {ssrc:#010x} (TTL: {ttl:?})");

      tokio::select! {
          _ = tok.cancelled() => {
              sleep(Duration::from_millis(200)).await;
              println!("Zatrzymano nadzór SSRC {ssrc:#010x}");
          }
          _ = sleep(ttl) => {
              println!("Klucz dla SSRC {ssrc:#010x} wygasł");
          }
      }
    });
  }

  let _ = signal::ctrl_c().await;
  tracker.close();
  token.cancel();
  let _ = timeout(Duration::from_secs(5), tracker.wait()).await;
}

/// Zadanie 6: nasłuchuj datagramów UDP na `listen_addr`, odkrywaj dynamicznie SSRC
/// przez `rtp_parser::parse_rtp_header`, pobieraj klucz z KMS (`kms_client::fetch_key`),
/// rejestruj strumień w `control-plane` (`ControlClient::register_stream`), dystrybuuj
/// ramki przez kanały `broadcast::<Bytes>` do konsumenta `--forward`, wolnego konsumenta
/// (`RecvError::Lagged`) i zadania TTL, a po `Ctrl+C` wykonaj *graceful shutdown*.
pub async fn run_probe(
  kms_url: &str,
  control: &ControlClient,
  listen_addr: SocketAddr,
  forward_addr: Option<SocketAddr>,
) -> std::io::Result<()> {
  let socket = UdpSocket::bind(listen_addr).await?;
  println!("edge-probe: nasłuchuję datagramów RTP na {listen_addr}");

  let forward_socket = match forward_addr {
    Some(target) => {
      let bind_addr = if target.is_ipv4() {
        "0.0.0.0:0"
      } else {
        "[::]:0"
      };
      Some((Arc::new(UdpSocket::bind(bind_addr).await?), target))
    }
    None => None,
  };

  let tracker = TaskTracker::new();
  let token = CancellationToken::new();

  let (alerts_tx, mut alerts_rx) = mpsc::channel::<SlaAlertPayload>(32);
  let reporter_client = control.clone();
  tracker.spawn(async move {
    while let Some(alert) = alerts_rx.recv().await {
      if let Err(err) = reporter_client.send_alert(&alert).await {
        eprintln!("Błąd wysyłki alertu dla SSRC {:#010x}: {err}", alert.ssrc);
      }
    }
  });

  let mut streams: HashMap<u32, broadcast::Sender<Bytes>> = HashMap::new();
  let mut buf = vec![0u8; 2048];

  loop {
    tokio::select! {
        _ = signal::ctrl_c() => {
            println!("edge-probe: otrzymano Ctrl+C, zamykanie...");
            break;
        }
        recv_res = socket.recv_from(&mut buf) => {
            let (n, peer) = recv_res?;
            let frame = Bytes::copy_from_slice(&buf[..n]);
            let Ok((header, _)) = parse_rtp_header(frame.clone()) else {
                continue;
            };
            let ssrc = header.ssrc;

            if !streams.contains_key(&ssrc) {
                let key = match kms_client::fetch_key(kms_url, ssrc).await {
                    Ok(key) => key,
                    Err(err) => {
                        eprintln!("Brak klucza w KMS dla SSRC {ssrc:#010x} (od {peer}): {err}");
                        continue;
                    }
                };

                let stream = StreamInfo {
                    ssrc,
                    label: format!("stream-{}-{}-{ssrc:#010x}", peer.ip(), peer.port()),
                    egress_addr: forward_addr.map(|addr| addr.to_string()),
                };
                if let Err(err) = control.register_stream(&stream).await {
                    eprintln!("Błąd rejestracji strumienia {ssrc:#010x} w control-plane: {err}");
                }

                let (tx, _) = broadcast::channel::<Bytes>(16);
                streams.insert(ssrc, tx.clone());

                spawn_ttl_task(&tracker, &token, &alerts_tx, ssrc, key);
                spawn_forward_consumer(&tracker, &tx, ssrc, forward_socket.clone());

                if streams.len() == 1 {
                    spawn_slow_consumer(&tracker, &tx, ssrc);
                }
            }

            if let Some(tx) = streams.get(&ssrc) {
                let _ = tx.send(frame);
            }
        }
    }
  }

  drop(streams);
  drop(alerts_tx);
  tracker.close();
  token.cancel();
  let _ = timeout(Duration::from_secs(5), tracker.wait()).await;
  Ok(())
}

fn spawn_ttl_task(
  tracker: &TaskTracker,
  token: &CancellationToken,
  alerts_tx: &mpsc::Sender<SlaAlertPayload>,
  ssrc: u32,
  key: KmsKeyResponse,
) {
  let tok = token.clone();
  let alerts_tx = alerts_tx.clone();
  tracker.spawn(async move {
    let ttl = (key.expires_at - Utc::now())
      .to_std()
      .unwrap_or(Duration::ZERO);
    println!("Start nadzoru SSRC {ssrc:#010x} (TTL: {ttl:?})");

    tokio::select! {
        _ = tok.cancelled() => {
            sleep(Duration::from_millis(200)).await;
            println!("Zatrzymano nadzór SSRC {ssrc:#010x}");
        }
        _ = sleep(ttl) => {
            println!("Klucz dla SSRC {ssrc:#010x} wygasł");
            let _ = alerts_tx
                .send(SlaAlertPayload {
                    ssrc,
                    fraction_lost: 0.0,
                    reason: "key_expired".into(),
                })
                .await;
        }
    }
  });
}

fn spawn_forward_consumer(
  tracker: &TaskTracker,
  tx: &broadcast::Sender<Bytes>,
  ssrc: u32,
  forward_socket: Option<(Arc<UdpSocket>, SocketAddr)>,
) {
  let mut forward_rx = tx.subscribe();
  tracker.spawn(async move {
        let mut total_bytes = 0usize;
        let mut total_frames = 0u64;
        let target = forward_socket
            .as_ref()
            .map(|(_, addr)| addr.to_string())
            .unwrap_or_else(|| "brak (--forward niepodane)".to_string());
        loop {
            match forward_rx.recv().await {
                Ok(frame) => {
                    total_bytes += frame.len();
                    total_frames += 1;
                    if let Some((sock, addr)) = &forward_socket {
                        let _ = sock.send_to(&frame, *addr).await;
                    }
                }
                Err(RecvError::Lagged(skipped)) => {
                    eprintln!(
                        "Konsument wyjściowy ({ssrc:#010x} -> {target}) pominął {skipped} ramek"
                    );
                }
                Err(RecvError::Closed) => {
                    println!(
                        "Konsument wyjściowy ({ssrc:#010x} -> {target}) zakończył ({total_frames} ramek, {total_bytes} B)"
                    );
                    break;
                }
            }
        }
    });
}

fn spawn_slow_consumer(
  tracker: &TaskTracker,
  tx: &broadcast::Sender<Bytes>,
  ssrc: u32,
) {
  let mut slow_rx = tx.subscribe();
  tracker.spawn(async move {
    loop {
      match slow_rx.recv().await {
        Ok(_frame) => {
          sleep(Duration::from_millis(250)).await;
        }
        Err(RecvError::Lagged(skipped)) => {
          println!("Wolny konsument ({ssrc:#010x}) pominął {skipped} ramek");
        }
        Err(RecvError::Closed) => break,
      }
    }
  });
}
