use probe_core::orchestrator::{run_stream_orchestrator, with_retry};
use std::time::Duration;

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

