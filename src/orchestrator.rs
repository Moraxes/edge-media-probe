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
pub async fn with_retry<T, E, F>(mut op: F, max_retries: usize) -> Result<T, E>
where
    F: AsyncFnMut() -> Result<T, E>,
{
    let mut attempt = 0;
    loop {
        match op().await {
            Ok(val) => return Ok(val),
            Err(_err) if attempt < max_retries => {
                attempt += 1;
                tokio::time::sleep(Duration::from_millis(50 * attempt as u64)).await;
            }
            Err(err) => return Err(err),
        }
    }
}

/// Przykładowy orkiestrator nadzorujący zadaną liczbę strumieni
pub async fn run_stream_orchestrator(stream_count: u32, timeout_dur: Duration) -> Vec<ProbeWorkerReport> {
    let mut workers: JoinSet<Result<ProbeWorkerReport, &'static str>> = JoinSet::new();
    let mut reports = Vec::new();

    for idx in 1..=stream_count {
        let ssrc = 0x1000_0000 + idx;
        let sleep_ms = 15 * idx as u64;
        workers.spawn(async move {
            with_retry(
                async move || {
                    tokio::time::sleep(Duration::from_millis(sleep_ms)).await;
                    Ok(ProbeWorkerReport {
                        stream_ssrc: ssrc,
                        status: "ACTIVE_STREAM",
                    })
                },
                3,
            )
            .await
        });
    }

    let global_deadline = tokio::time::sleep(timeout_dur);
    tokio::pin!(global_deadline);

    loop {
        if workers.is_empty() {
            break;
        }
        tokio::select! {
            res = workers.join_next() => {
                match res {
                    Some(Ok(Ok(report))) => reports.push(report),
                    Some(_) => {},
                    None => break,
                }
            }
            _ = &mut global_deadline => {
                workers.abort_all();
                break;
            }
            _ = tokio::signal::ctrl_c() => {
                workers.abort_all();
                break;
            }
        }
    }

    reports
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_with_retry_succeeds_after_transient_failure() {
        let mut attempts = 0;
        let res: Result<&str, &str> = with_retry(
            async || {
                attempts += 1;
                if attempts < 3 {
                    Err("transient error")
                } else {
                    Ok("success")
                }
            },
            3,
        )
        .await;

        assert_eq!(res, Ok("success"));
        assert_eq!(attempts, 3);
    }

    #[tokio::test]
    async fn test_orchestrator_collects_reports() {
        let reports = run_stream_orchestrator(3, Duration::from_secs(2)).await;
        assert_eq!(reports.len(), 3);
    }
}
