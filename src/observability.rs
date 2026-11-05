//! Moduł 2 (Zadania 2.1, 2.2, 2.3): Obserwowalność węzła brzegowego.
//!
//! Ćwiczenie obejmuje:
//! - Zadanie 2.1: Diagnostykę egzekutora Tokio w `tokio-console`,
//! - Zadanie 2.2: Ustrukturyzowane logowanie JSON i zdarzenia anomalii (`record_rtp_gap_event`),
//! - Zadanie 2.3: Atomowe metryki Prometheusa (`ProbeDataplaneMetrics`) i strażnika RAII `Drop` (`ActiveResourceGuard`).

use metrics::{counter, gauge, histogram};
use std::time::Duration;
use tracing::warn;

/// Strażnik RAII gwarantujący Cancellation Safety dla metryk zasobów w locie
pub struct ActiveResourceGuard {
    metric_name: &'static str,
}

impl ActiveResourceGuard {
    pub fn new(metric_name: &'static str) -> Self {
        gauge!(metric_name).increment(1.0);
        Self { metric_name }
    }
}

impl Drop for ActiveResourceGuard {
    fn drop(&mut self) {
        gauge!(self.metric_name).decrement(1.0);
    }
}

/// Bezalokacyjna fasada gorącej ścieżki metryk (100% pakietów)
pub struct ProbeDataplaneMetrics;

impl ProbeDataplaneMetrics {
    #[inline]
    pub fn on_packet(kind: &'static str, payload_len: usize, interarrival_us: f64) {
        counter!("probe_packets_total", "kind" => kind).increment(1);
        counter!("probe_bytes_total", "kind" => kind).increment(payload_len as u64);
        histogram!("probe_interarrival_jitter_us", "kind" => kind).record(interarrival_us);
    }

    #[inline]
    pub fn on_rtp_gap_detected(missing_packets: usize) {
        counter!("probe_rtp_packets_lost_total").increment(missing_packets as u64);
        counter!("probe_rtcp_nack_requests_total").increment(1);
    }

    #[inline]
    pub fn on_srtp_auth_error() {
        counter!("probe_srtp_auth_failures_total").increment(1);
    }
}

/// Zimna ścieżka telemetrii: rejestrowanie rzadkich anomalii w drzewie spanów
pub fn record_rtp_gap_event(ssrc: u32, missing_seqs: &[u16], roc: u32) {
    warn!(
        ssrc = %format!("{ssrc:#010x}"),
        roc,
        missing_count = missing_seqs.len(),
        ?missing_seqs,
        "Wykryto lukę w strumieniu RTP – żądanie RTCP NACK"
    );
}

/// Cykliczny kolektor metryk egzekutora Tokio
pub fn spawn_runtime_metrics_collector() {
    tokio::spawn(async move {
        let handle = tokio::runtime::Handle::current();
        let mut interval = tokio::time::interval(Duration::from_secs(5));
        loop {
            interval.tick().await;
            let rt_metrics = handle.metrics();
            for worker in 0..rt_metrics.num_workers() {
                let w_label = worker.to_string();
                #[cfg(tokio_unstable)]
                {
                    gauge!("tokio_worker_steal_count", "worker" => w_label.clone())
                        .set(rt_metrics.worker_steal_count(worker) as f64);
                    gauge!("tokio_worker_local_queue_depth", "worker" => w_label)
                        .set(rt_metrics.worker_local_queue_depth(worker) as f64);
                }
                #[cfg(not(tokio_unstable))]
                {
                    gauge!("tokio_worker_park_count", "worker" => w_label)
                        .set(rt_metrics.worker_park_count(worker) as f64);
                }
            }
        }
    });
}
