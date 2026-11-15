//! Konfiguracja nieblokującej telemetrii (`tracing-appender`) oraz `syslog-tracing`.

use tracing_appender::non_blocking::WorkerGuard;

/// Zadanie 8: zainicjalizuj wielowarstwowy `tracing_subscriber::Registry`:
/// 1. `EnvFilter` z `RUST_LOG` (domyślnie `info`),
/// 2. nieblokującą warstwę JSON na `stdout` (`tracing_appender::non_blocking`),
/// 3. opcjonalną warstwę `syslog_tracing::Syslog` z filtrem `LevelFilter::WARN` (Unix).
/// Zwróć `WorkerGuard`, aby utrzymać wątek zapisu do końca funkcji `main()`.
pub fn init_tracing(
  _service_name: &'static str,
) -> Result<WorkerGuard, Box<dyn std::error::Error>> {
  todo!(
    "Zadanie 8: skonfiguruj warstwy EnvFilter, non_blocking JSON oraz syslog-tracing"
  )
}
