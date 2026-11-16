//! Nadzór nad zadaniami strumieni i ich bezpieczne wygaszanie (*graceful shutdown*).

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
//
use bytes::{Bytes, BytesMut};
use chrono::Utc;
use rtp_parser::{PacketClass, parse_rtp_header};
use tokio::net::UdpSocket;
use tokio::signal;
use tokio::sync::broadcast::error::RecvError;
use tokio::sync::{broadcast, mpsc};
use tokio::time::{sleep, timeout};
use tokio_stream::wrappers::BroadcastStream;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;
//
use crate::control_client::ControlClient;
use crate::control_plane::{SlaAlertPayload, StreamInfo};
use crate::kms_client::{KmsClient, KmsKeyResponse};
use crate::stream_pipeline;
use crate::udp_ingress;

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

struct StreamEntry {
  tx: broadcast::Sender<Bytes>,
}

#[derive(Clone)]
struct WorkerConfig {
  kms: KmsClient,
  control: ControlClient,
  forward_socket: Arc<UdpSocket>,
  forward_addr: SocketAddr,
  alerts_tx: mpsc::Sender<SlaAlertPayload>,
  token: CancellationToken,
}

struct WorkerState {
  cfg: WorkerConfig,
  tracker: TaskTracker,
  streams: HashMap<u32, StreamEntry>,
  pool: BytesMut,
}

impl WorkerState {
  fn new(cfg: WorkerConfig) -> Self {
    Self {
      cfg,
      tracker: TaskTracker::new(),
      streams: HashMap::new(),
      pool: BytesMut::with_capacity(4096),
    }
  }

  async fn handle_rtcp(&mut self, frame: Bytes) {
    let _ = self
      .cfg
      .forward_socket
      .send_to(&frame, self.cfg.forward_addr)
      .await;
  }

  async fn ensure_stream(
    &mut self,
    ssrc: u32,
    peer: SocketAddr,
  ) -> Option<&mut StreamEntry> {
    if self.streams.contains_key(&ssrc) {
      return self.streams.get_mut(&ssrc);
    }

    let key = match self.cfg.kms.fetch_key(ssrc).await {
      Ok(key) => key,
      Err(err) => {
        eprintln!("Brak klucza w KMS dla SSRC {ssrc:#010x} (od {peer}): {err}");
        return None;
      }
    };

    let stream = StreamInfo {
      ssrc,
      label: format!("stream-{}-{}-{ssrc:#010x}", peer.ip(), peer.port()),
      egress_addr: Some(self.cfg.forward_addr.to_string()),
    };
    if let Err(err) = self
      .cfg
      .control
      .register_stream(&stream)
      .await
    {
      eprintln!(
        "Błąd rejestracji strumienia {ssrc:#010x} w control-plane: {err}"
      );
    }

    let (tx, _) = broadcast::channel::<Bytes>(16);
    self
      .streams
      .insert(ssrc, StreamEntry { tx: tx.clone() });

    spawn_ttl_task(
      &self.tracker,
      &self.cfg.token,
      &self.cfg.alerts_tx,
      ssrc,
      key,
    );
    spawn_forward_consumer(
      &self.tracker,
      &tx,
      ssrc,
      self.cfg.forward_socket.clone(),
      self.cfg.forward_addr,
    );
    spawn_sla_analyzer(
      &self.tracker,
      &self.cfg.token,
      &self.cfg.alerts_tx,
      &tx,
      ssrc,
    );

    if self.streams.len() == 1 {
      spawn_slow_consumer(&self.tracker, &tx, ssrc);
    }

    self.streams.get_mut(&ssrc)
  }

  async fn handle_rtp(&mut self, frame: Bytes, peer: SocketAddr) {
    let Ok((header, _)) = parse_rtp_header(frame.clone()) else {
      return;
    };
    let Some(entry) = self
      .ensure_stream(header.ssrc, peer)
      .await
    else {
      return;
    };
    let _ = entry.tx.send(frame);
  }
}

async fn bind_forward_socket(
  forward_addr: SocketAddr,
) -> std::io::Result<Arc<UdpSocket>> {
  let bind_addr = if forward_addr.is_ipv4() {
    "0.0.0.0:0"
  } else {
    "[::]:0"
  };
  Ok(Arc::new(UdpSocket::bind(bind_addr).await?))
}

fn spawn_alerts_reporter(
  tracker: &TaskTracker,
  control: &ControlClient,
) -> mpsc::Sender<SlaAlertPayload> {
  let (alerts_tx, mut alerts_rx) = mpsc::channel::<SlaAlertPayload>(32);
  let reporter_client = control.clone();
  tracker.spawn(async move {
    while let Some(alert) = alerts_rx.recv().await {
      if let Err(err) = reporter_client.send_alert(&alert).await {
        eprintln!("Błąd wysyłki alertu dla SSRC {:#010x}: {err}", alert.ssrc);
      }
    }
  });
  alerts_tx
}

async fn run_worker_loop(
  socket: UdpSocket,
  cfg: WorkerConfig,
) -> std::io::Result<()> {
  let token = cfg.token.clone();
  let mut state = WorkerState::new(cfg);

  loop {
    tokio::select! {
      _ = token.cancelled() => break,
      recv_res = udp_ingress::recv_rtp_or_rtcp(&socket, &mut state.pool) => {
        let (class, frame, peer) = recv_res?;
        match class {
          Some(PacketClass::Rtcp) => state.handle_rtcp(frame).await,
          Some(PacketClass::Rtp) => state.handle_rtp(frame, peer).await,
          None => {}
        }
      }
    }
  }

  drop(state.streams);
  drop(state.cfg);
  state.tracker.close();
  let _ = timeout(Duration::from_secs(5), state.tracker.wait()).await;
  Ok(())
}

async fn serve_sockets(
  sockets: Vec<UdpSocket>,
  kms: &KmsClient,
  control: &ControlClient,
  forward_addr: SocketAddr,
) -> std::io::Result<()> {
  let forward_socket = bind_forward_socket(forward_addr).await?;
  let tracker = TaskTracker::new();
  let token = CancellationToken::new();
  let alerts_tx = spawn_alerts_reporter(&tracker, control);

  let base_cfg = WorkerConfig {
    kms: kms.clone(),
    control: control.clone(),
    forward_socket,
    forward_addr,
    alerts_tx: alerts_tx.clone(),
    token: token.clone(),
  };

  for (worker_id, socket) in sockets.into_iter().enumerate() {
    let cfg = base_cfg.clone();
    tracker.spawn(async move {
      if let Err(err) = run_worker_loop(socket, cfg).await {
        eprintln!("Błąd workera {worker_id}: {err}");
      }
    });
  }
  drop(base_cfg);

  let _ = signal::ctrl_c().await;
  println!("edge-probe: otrzymano Ctrl+C, zamykanie...");
  drop(alerts_tx);
  token.cancel();
  tracker.close();
  let _ = timeout(Duration::from_secs(5), tracker.wait()).await;
  Ok(())
}

/// Zadanie 6: nasłuchuj datagramów UDP na `listen_addr`, odkrywaj dynamicznie SSRC
/// przez `rtp_parser::parse_rtp_header`, pobieraj klucz z KMS (`KmsClient::fetch_key`),
/// rejestruj strumień w `control-plane` (`ControlClient::register_stream`), dystrybuuj
/// ramki przez kanały `broadcast::<Bytes>` do konsumenta `--forward`, wolnego konsumenta
/// (`RecvError::Lagged`) i zadania TTL, a po `Ctrl+C` wykonaj *graceful shutdown*.
pub async fn run_probe(
  kms: &KmsClient,
  control: &ControlClient,
  listen_addr: SocketAddr,
  forward_addr: SocketAddr,
) -> std::io::Result<()> {
  let socket = udp_ingress::bind_tuned_udp(listen_addr, 262_144)?;
  println!("edge-probe: nasłuchuję datagramów RTP na {listen_addr}");
  serve_sockets(vec![socket], kms, control, forward_addr).await
}

/// Zadanie 12: uruchom `workers` niezależnych pętli workerów bez współdzielonego
/// `Mutex`, każdą z własnym gniazdem `udp_ingress::bind_reuseport_udp`, lokalną
/// areną `BytesMut::with_capacity(4096)` i własnym zestawem map per-worker.
pub async fn run_probe_workers(
  kms: &KmsClient,
  control: &ControlClient,
  listen_addr: SocketAddr,
  forward_addr: SocketAddr,
  workers: usize,
) -> std::io::Result<()> {
  todo!()
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
  forward_socket: Arc<UdpSocket>,
  forward_addr: SocketAddr,
) {
  let mut forward_rx = tx.subscribe();
  tracker.spawn(async move {
    let mut total_bytes = 0usize;
    let mut total_frames = 0u64;
    loop {
      match forward_rx.recv().await {
        Ok(frame) => {
          total_bytes += frame.len();
          total_frames += 1;
          let _ = forward_socket.send_to(&frame, forward_addr).await;
        }
        Err(RecvError::Lagged(skipped)) => {
          eprintln!(
            "Konsument wyjściowy ({ssrc:#010x} -> {forward_addr}) pominął {skipped} ramek"
          );
        }
        Err(RecvError::Closed) => {
          println!(
            "Konsument wyjściowy ({ssrc:#010x} -> {forward_addr}) zakończył ({total_frames} ramek, {total_bytes} B)"
          );
          break;
        }
      }
    }
  });
}

fn spawn_sla_analyzer(
  tracker: &TaskTracker,
  token: &CancellationToken,
  alerts_tx: &mpsc::Sender<SlaAlertPayload>,
  tx: &broadcast::Sender<Bytes>,
  ssrc: u32,
) {
  let rx = BroadcastStream::new(tx.subscribe());
  let alerts_tx = alerts_tx.clone();
  let tok = token.clone();
  tracker.spawn(async move {
    stream_pipeline::run_sla_analyzer(ssrc, rx, alerts_tx, tok).await;
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
