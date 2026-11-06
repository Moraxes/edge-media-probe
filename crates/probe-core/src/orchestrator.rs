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
    // Zadanie 1.1: Zaimplementuj mechanizm ponawiania prób z wykorzystaniem cechy AsyncFnMut (Rust 2024).
    // Wymagania:
    // 1. Wykonaj asynchroniczną operację `op().await`.
    // 2. Jeśli operacja zwróci błąd i nie przekroczono `max_retries`, odczekaj czas zależny od próby
    //    (np. 50 ms * numer próby za pomocą `tokio::time::sleep`) i spróbuj ponownie.
    // 3. Po wyczerpaniu limitu zwróć ostatni napotkany błąd.
    todo!("Zadanie 1.1: Zaimplementuj with_retry z backoffem opartym na AsyncFnMut")
}

/// Przykładowy orkiestrator nadzorujący zadaną liczbę strumieni
pub async fn run_stream_orchestrator(_stream_count: u32, _timeout_dur: Duration) -> Vec<ProbeWorkerReport> {
    // Zadanie 1.1: Zaimplementuj orkiestrator zadań w Tokio:
    // Wymagania:
    // 1. Utwórz `JoinSet` dla zadań roboczych.
    // 2. Uruchom `stream_count` zadań (spawn) z unikalnym SSRC (np. 0x1000_0000 + idx) i ponawianiem prób przez `with_retry`.
    // 3. Zastosuj pętlę `tokio::select!` obsługującą:
    //    - `workers.join_next()`,
    //    - globalny timeout przypięty na stosie (`tokio::pin!`),
    //    - sygnał wygaszenia `tokio::signal::ctrl_c()`.
    // 4. Zwróć zebrane raporty `ProbeWorkerReport`.
    todo!("Zadanie 1.1: Zaimplementuj run_stream_orchestrator z JoinSet i select!")
}
