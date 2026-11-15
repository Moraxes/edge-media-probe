//! Metryki gorącej ścieżki data plane (`metrics`) oraz eksporter Prometheusa (`GET /metrics`).

use std::net::SocketAddr;

/// Strażnik RAII zliczający aktywne strumienie RTP w `probe_active_streams`.
/// Dekrementuje wskaźnik w `Drop`, gwarantując poprawność przy anulowaniu zadania.
pub struct ActiveStreamGuard;

impl ActiveStreamGuard {
  pub fn new() -> Self {
    todo!("Zadanie 9: zwiększ gauge!(\"probe_active_streams\") o 1.0")
  }
}

impl Default for ActiveStreamGuard {
  fn default() -> Self {
    Self::new()
  }
}

impl Drop for ActiveStreamGuard {
  fn drop(&mut self) {
    todo!("Zadanie 9: zmniejsz gauge!(\"probe_active_streams\") o 1.0")
  }
}

/// Zadanie 9: zainstaluj rejestrator `PrometheusBuilder` i uruchom w tle serwer `axum`
/// wystawiający endpoint `GET /metrics` na podanym adresie (np. `127.0.0.1:3002`).
pub async fn init_prometheus_server(
  _bind_addr: SocketAddr,
) -> Result<(), Box<dyn std::error::Error>> {
  todo!("Zadanie 9: zainstaluj PrometheusBuilder i uruchom serwer GET /metrics")
}

/// Zadanie 9: zarejestruj pojedynczy pakiet RTP na gorącej ścieżce data plane
/// (`probe_packets_total`, `probe_bytes_total`, `probe_interarrival_jitter_us`).
/// Etykieta `kind` musi mieć stałą liczebność wariantów (np. `"rtp"`).
pub fn record_rtp_packet(_byte_len: usize, _jitter_us: f64) {
  todo!(
    "Zadanie 9: wywołaj counter! i histogram! ze stałą etykietą kind => rtp"
  )
}

/// Zadanie 9: zarejestruj wykryte straty pakietów oraz współczynnik utraty SLA w procentach.
pub fn record_rtp_loss(_lost_packets: u64, _loss_ratio_percent: f64) {
  todo!(
    "Zadanie 9: wywołaj counter!(\"probe_rtp_packets_lost_total\") i gauge!(\"probe_rtp_loss_ratio\")"
  )
}
