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
///
/// Obsługuje:
/// - Zmienną środowiskową `LOG_FORMAT=json` do emisji zdarzeń w standardzie Cloud-Native JSON,
/// - Filtr poziomów logowania `RUST_LOG` przez `EnvFilter`,
/// - Diagnostykę `tokio-console` przy kompilacji z `--cfg tokio_unstable`.
pub fn init_tracing(
    service_name: &'static str,
) -> Option<tracing_appender::non_blocking::WorkerGuard> {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,probe_core=debug"));

    // Nieblokujący writer (tracing-appender) na stdout zapobiega blokowaniu wątków roboczych Tokio:
    let (non_blocking, guard) = tracing_appender::non_blocking(std::io::stdout());

    let is_json = std::env::var("LOG_FORMAT")
        .map(|v| v.eq_ignore_ascii_case("json"))
        .unwrap_or(false);

    let fmt_layer = if is_json {
        fmt::layer()
            .json()
            .with_current_span(true)
            .with_span_list(true)
            .with_writer(non_blocking)
            .boxed()
    } else {
        fmt::layer()
            .with_target(true)
            .with_thread_ids(true)
            .with_writer(non_blocking)
            .boxed()
    };

    #[cfg(tokio_unstable)]
    let console_layer = {
        let (layer, server) = console_subscriber::ConsoleLayer::builder()
            .with_default_env()
            .build();

        std::thread::Builder::new()
            .name("console_subscriber".into())
            .spawn(move || {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build();
                if let Ok(runtime) = runtime {
                    runtime.block_on(async move {
                        if let Err(err) = server.serve().await {
                            tracing::warn!(
                                %err,
                                "Nie udało się uruchomić serwera gRPC tokio-console (np. port zajęty przez inną binarkę). Aby monitorować tę binarkę, ustaw TOKIO_CONSOLE_BIND=127.0.0.1:<inny_port>"
                            );
                        }
                    });
                }
            })
            .ok();

        Some(layer)
    };
    #[cfg(not(tokio_unstable))]
    let console_layer: Option<console_subscriber::ConsoleLayer> = None;

    let subscriber = Registry::default()
        .with(env_filter)
        .with(fmt_layer)
        .with(console_layer);

    let _ = subscriber.try_init();

    tracing::info!(
        service = service_name,
        json_format = is_json,
        tokio_unstable = cfg!(tokio_unstable),
        "Zainicjalizowano podsystem tracingu i telemetrii ustrukturyzowanej"
    );

    Some(guard)
}

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

    #[inline]
    pub fn on_rtcp_receiver_report(lost_packets: u32, extended_highest_seq: u32) {
        if extended_highest_seq > 0 {
            let ratio = (lost_packets as f64) / (extended_highest_seq as f64);
            gauge!("probe_rtp_loss_ratio").set((ratio * 100.0).clamp(0.0, 100.0));
        }
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
    use futures_util::StreamExt;
    use tokio_stream::wrappers::IntervalStream;

    tokio::spawn(async move {
        let handle = tokio::runtime::Handle::current();
        let interval = tokio::time::interval(Duration::from_secs(1));
        let stream = IntervalStream::new(interval);

        stream
            .for_each(move |_| {
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
                std::future::ready(())
            })
            .await;
    });
}
