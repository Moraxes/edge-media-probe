//! Moduł 2 (Zadania 2.1, 2.2, 2.3): Obserwowalność węzła brzegowego.
//!
//! Ćwiczenie obejmuje:
//! - Zadanie 2.1: Diagnostykę egzekutora Tokio w `tokio-console`,
//! - Zadanie 2.2: Ustrukturyzowane logowanie JSON i zdarzenia anomalii (`init_tracing`, `record_rtp_gap_event`),
//! - Zadanie 2.3: Atomowe metryki Prometheusa (`ProbeDataplaneMetrics`) i strażnika RAII `Drop` (`ActiveResourceGuard`).

use metrics::{counter, gauge, histogram};
use std::time::Duration;
use tracing::warn;
use tracing_subscriber::{
    fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer, Registry,
};

/// Inicjalizuje wielowarstwowy potok telemetrii (`tracing` + opcjonalnie `tokio-console`).
///
/// Zwraca strażnika `WorkerGuard`, który musi pozostać na stosie `main()`
/// aż do zakończenia programu, aby zapewnić bezblokujące opróżnienie buforów (`flush`).
pub fn init_tracing(
    _service_name: &'static str,
) -> Option<tracing_appender::non_blocking::WorkerGuard> {
    // Zadanie 2.2: Skonfiguruj wielowarstwowy potok telemetrii tracing (podpowiedź: git show hint-2.2)
    todo!("Zadanie 2.2: Zaimplementuj init_tracing z obsługą formatu JSON i warstw tracingu")
}

/// Strażnik RAII gwarantujący Cancellation Safety dla metryk zasobów w locie
pub struct ActiveResourceGuard {
    metric_name: &'static str,
}

impl ActiveResourceGuard {
    pub fn new(metric_name: &'static str) -> Self {
        // Zadanie 2.3: Inkrementacja metryki gauge w konstruktorze strażnika RAII (podpowiedź: git show hint-2.3)
        todo!("Zadanie 2.3: Zaimplementuj ActiveResourceGuard::new")
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
    pub fn on_packet(_kind: &'static str, _payload_len: usize, _interarrival_us: f64) {
        // Zadanie 2.3: Rejestracja atomowych metryk na gorącej ścieżce pakietów (podpowiedź: git show hint-2.3)
        todo!("Zadanie 2.3: Zaimplementuj rejestrację metryk on_packet")
    }

    #[inline]
    pub fn on_rtp_gap_detected(_missing_packets: usize) {
        // Zadanie 2.3: Zwiększenie liczników strat i żądań RTCP NACK (podpowiedź: git show hint-2.3)
        todo!("Zadanie 2.3: Zaimplementuj rejestrację metryk on_rtp_gap_detected")
    }

    #[inline]
    pub fn on_srtp_auth_error() {
        // Zadanie 2.3: Zwiększenie licznika błędów autentykacji SRTP (podpowiedź: git show hint-2.3)
        todo!("Zadanie 2.3: Zaimplementuj rejestrację metryk on_srtp_auth_error")
    }

    #[inline]
    pub fn on_rtcp_receiver_report(_lost_packets: u32, _extended_highest_seq: u32) {
        // Zadanie 2.3: Aktualizacja współczynnika strat w metryce gauge (podpowiedź: git show hint-2.3)
        todo!("Zadanie 2.3: Zaimplementuj rejestrację metryki on_rtcp_receiver_report")
    }
}

/// Zimna ścieżka telemetrii: rejestrowanie rzadkich anomalii w drzewie spanów
#[tracing::instrument(
    name = "rtp.anomaly",
    fields(ssrc = %format!("{ssrc:#010x}"), roc)
)]
pub fn record_rtp_gap_event(ssrc: u32, missing_seqs: &[u16], roc: u32) {
    warn!(
        roc,
        missing_count = missing_seqs.len(),
        ?missing_seqs,
        "Wykryto lukę w strumieniu RTP – żądanie RTCP NACK"
    );
}

/// Cykliczny kolektor metryk egzekutora Tokio (oparty na kombinatorze IntervalStream z tokio-stream)
pub fn spawn_runtime_metrics_collector() {
    // Zadanie 2.1: Cykliczny zrzut metryk wykonawcy Tokio przez IntervalStream (podpowiedź: git show hint-2.1)
    todo!("Zadanie 2.1: Zaimplementuj spawn_runtime_metrics_collector")
}
