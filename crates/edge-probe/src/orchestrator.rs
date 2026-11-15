//! Nadzór nad zadaniami strumieni i ich bezpieczne wygaszanie (*graceful shutdown*).

use std::collections::HashMap;
use std::net::SocketAddr;
//
use crate::control_client::ControlClient;
use crate::kms_client::KmsKeyResponse;

/// Zadanie 3: uruchom w `TaskTracker` zadania dla kluczy z Zadania 2
/// i przeprowadź *graceful shutdown* po otrzymaniu `Ctrl+C`.
pub async fn supervise_keys(_keys: HashMap<u32, KmsKeyResponse>) {
  todo!()
}

/// Zadanie 6: nasłuchuj datagramów UDP na `listen_addr`, odkrywaj dynamicznie SSRC
/// przez `rtp_parser::parse_rtp_header`, pobieraj klucz z KMS (`kms_client::fetch_key`),
/// rejestruj strumień w `control-plane` (`ControlClient::register_stream`), dystrybuuj
/// ramki przez kanały `broadcast::<Bytes>` do konsumenta `--forward`, wolnego konsumenta
/// (`RecvError::Lagged`) i zadania TTL, a po `Ctrl+C` wykonaj *graceful shutdown*.
pub async fn run_probe(
  _kms_url: &str,
  _control: &ControlClient,
  _listen_addr: SocketAddr,
  _forward_addr: Option<SocketAddr>,
) -> std::io::Result<()> {
  todo!()
}

/// Zadanie 12: uruchom `workers` niezależnych pętli workerów bez współdzielonego
/// `Mutex`, każdą z własnym gniazdem `udp_ingress::bind_reuseport_udp`, lokalną
/// areną `BytesMut::with_capacity(4096)` i własnym zestawem map per-worker.
pub async fn run_probe_workers(
  _kms_url: &str,
  _control: &ControlClient,
  _listen_addr: SocketAddr,
  _forward_addr: Option<SocketAddr>,
  _workers: usize,
) -> std::io::Result<()> {
  todo!()
}
