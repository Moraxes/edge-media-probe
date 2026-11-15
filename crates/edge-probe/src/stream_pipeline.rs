//! Asynchroniczny potok analizy ciągłości ramek RTP i wykrywania anomalii SLA (`Stream`).

use std::future;
use std::time::Duration;
//
use bytes::Bytes;
use futures::{Stream, TryStreamExt};
use rtp_parser::{RtpHeader, RtpParseError, parse_rtp_header};
use thiserror::Error;
use tokio::sync::mpsc;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::errors::BroadcastStreamRecvError;
use tokio_util::sync::CancellationToken;
//
use crate::control_plane::SlaAlertPayload;

#[derive(Debug, Error)]
enum PipelineError {
  #[error("przekroczono czas oczekiwania na ramkę RTP")]
  FrameTimeout,
  #[error("odbiorca broadcast pominął {0} ramek")]
  Lagged(u64),
  #[error("błąd parsowania RTP: {0}")]
  Parse(#[from] RtpParseError),
}

#[derive(Debug, Default, Clone, Copy)]
struct SeqObservation {
  received: u64,
  lost: u64,
  timed_out: bool,
}

/// Zadanie 7: potok analizy ciągłości pakietów RTP w oparciu o kombinatory `Stream`
/// (`take_until`, `timeout`, normalizacja `Elapsed`/`BroadcastStreamRecvError`, `try_filter_map`
/// z `rtp_parser::parse_rtp_header`, `scan`, `chunks_timeout`) publikujący wykryte luki
/// sekwencyjne jako `SlaAlertPayload` do kanału `alerts_tx`.
pub async fn run_sla_analyzer<S>(
  ssrc: u32,
  frames: S,
  alerts_tx: mpsc::Sender<SlaAlertPayload>,
  shutdown: CancellationToken,
) where
  S: Stream<Item = Result<Bytes, BroadcastStreamRecvError>> + Unpin,
{
  let parsed_headers =
    futures::StreamExt::take_until(frames, shutdown.cancelled_owned())
      .timeout(Duration::from_millis(500))
      .map(|res| match res {
        Ok(Ok(frame)) => Ok(frame),
        Ok(Err(BroadcastStreamRecvError::Lagged(skipped))) => {
          Err(PipelineError::Lagged(skipped))
        }
        Err(_) => Err(PipelineError::FrameTimeout),
      })
      .try_filter_map(|frame| async move {
        let (header, _payload) = parse_rtp_header(frame)?;
        Ok((header.ssrc == ssrc).then_some(header))
      });

  let observations = futures::StreamExt::scan(
    parsed_headers,
    None::<u16>,
    |last_seq, item: Result<RtpHeader, PipelineError>| {
      let obs = match item {
        Ok(header) => {
          let seq = header.sequence_number;
          let lost = match *last_seq {
            Some(prev) => {
              let delta = seq.wrapping_sub(prev);
              if delta > 1 && delta < 0x8000 {
                (delta - 1) as u64
              } else {
                0
              }
            }
            None => 0,
          };
          *last_seq = Some(seq);
          SeqObservation {
            received: 1,
            lost,
            timed_out: false,
          }
        }
        Err(PipelineError::Lagged(skipped)) => {
          *last_seq = None;
          SeqObservation {
            received: 0,
            lost: skipped,
            timed_out: false,
          }
        }
        Err(PipelineError::FrameTimeout) => SeqObservation {
          received: 0,
          lost: 0,
          timed_out: true,
        },
        Err(PipelineError::Parse(err)) => {
          eprintln!("Błąd parsowania pakietu SSRC {ssrc:#010x}: {err}");
          SeqObservation::default()
        }
      };
      future::ready(Some(obs))
    },
  );

  let alert_windows = observations
    .chunks_timeout(50, Duration::from_secs(1))
    .filter_map(|window| {
      let mut received = 0u64;
      let mut lost = 0u64;
      let mut timeouts = 0u64;
      for obs in window {
        received += obs.received;
        lost += obs.lost;
        if obs.timed_out {
          timeouts += 1;
        }
      }

      if lost == 0 && timeouts == 0 {
        return None;
      }

      let expected = (received + lost).max(1);
      let fraction_lost = lost as f32 / expected as f32;
      let reason = if lost > 0 {
        format!("seq_gap_lost_{lost}")
      } else {
        format!("frame_timeout_{timeouts}")
      };

      Some(SlaAlertPayload {
        ssrc,
        fraction_lost,
        reason,
      })
    });

  futures::StreamExt::for_each(alert_windows, |alert| {
    let tx = alerts_tx.clone();
    async move {
      let _ = tx.send(alert).await;
    }
  })
  .await;
}

#[cfg(test)]
mod tests {
  use super::*;
  use rtp_parser::build_test_rtp_packet;

  #[tokio::test]
  async fn detects_sequence_gap_in_rtp_stream() {
    let ssrc = 0x1000;
    let packets = vec![1u16, 2, 5, 6]
      .into_iter()
      .map(|seq| {
        Ok::<_, BroadcastStreamRecvError>(build_test_rtp_packet(
          seq,
          u32::from(seq) * 3000,
          ssrc,
          b"payload",
        ))
      });

    let (alerts_tx, mut alerts_rx) = mpsc::channel(8);
    let shutdown = CancellationToken::new();

    run_sla_analyzer(ssrc, futures::stream::iter(packets), alerts_tx, shutdown)
      .await;

    let alert = alerts_rx
      .recv()
      .await
      .expect("oczekiwano alertu o luce sekwencyjnej");
    assert_eq!(alert.ssrc, ssrc);
    assert_eq!(alert.reason, "seq_gap_lost_2");
    assert!((alert.fraction_lost - (2.0 / 6.0)).abs() < 1e-5);
    assert!(alerts_rx.recv().await.is_none());
  }

  #[tokio::test]
  async fn handles_u16_wraparound_and_lagged_error() {
    let ssrc = 0x1001;
    let packets = vec![
      Ok(build_test_rtp_packet(65534, 1000, ssrc, b"a")),
      Ok(build_test_rtp_packet(65535, 4000, ssrc, b"b")),
      Ok(build_test_rtp_packet(0, 7000, ssrc, b"c")),
      Err(BroadcastStreamRecvError::Lagged(3)),
      Ok(build_test_rtp_packet(4, 19000, ssrc, b"d")),
    ];

    let (alerts_tx, mut alerts_rx) = mpsc::channel(8);
    let shutdown = CancellationToken::new();

    run_sla_analyzer(ssrc, futures::stream::iter(packets), alerts_tx, shutdown)
      .await;

    let alert = alerts_rx
      .recv()
      .await
      .expect("oczekiwano alertu dla błędu Lagged");
    assert_eq!(alert.ssrc, ssrc);
    assert_eq!(alert.reason, "seq_gap_lost_3");
    assert!((alert.fraction_lost - (3.0 / 7.0)).abs() < 1e-5);
  }

  #[tokio::test]
  async fn stops_on_cancellation_token() {
    let ssrc = 0x1002;
    let (alerts_tx, _alerts_rx) = mpsc::channel(8);
    let shutdown = CancellationToken::new();
    let tok = shutdown.clone();

    let handle = tokio::spawn(async move {
      run_sla_analyzer(ssrc, futures::stream::pending(), alerts_tx, tok).await;
    });

    shutdown.cancel();
    tokio::time::timeout(Duration::from_secs(1), handle)
      .await
      .expect("analizator powinien zakończyć się po anulowaniu")
      .expect("zadanie nie powinno panikować");
  }
}
