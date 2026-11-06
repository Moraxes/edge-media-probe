//! Moduł 1 (Zadanie 1.1): Wielozadaniowy orkiestrator strumieni.
//!
//! Ćwiczenie obejmuje:
//! - Wzorzec ponawiania prób `with_retry` oparty na traicie `AsyncFnMut` (Rust 1.85+ / 2024),
//! - Zarządzanie równoległymi zadaniami przez `tokio::task::JoinSet`,
//! - Pętlę `tokio::select!` z przypiętym na stosie timeoutem (`tokio::pin!`) oraz wygaszaniem `ctrl_c`.

use std::time::Duration;
use tokio::task::JoinSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeWorkerReport {
    pub stream_ssrc: u32,
    pub status: &'static str,
}

/// Uniwersalna funkcja ponawiania prób wykorzystująca asynchroniczne domknięcia `AsyncFnMut`
pub async fn with_retry<T, E, F>(mut _op: F, _max_retries: usize) -> Result<T, E>
where
    F: AsyncFnMut() -> Result<T, E>,
{
    // Zadanie 1.1: Zaimplementuj with_retry z backoffem
    todo!()
}

/// Przykładowy orkiestrator nadzorujący zadaną liczbę strumieni
pub async fn run_stream_orchestrator(_stream_count: u32, _timeout_dur: Duration) -> Vec<ProbeWorkerReport> {
    // Zadanie 1.1: Zaimplementuj nadzór nad zadaniami w JoinSet i select!
    todo!()
}
