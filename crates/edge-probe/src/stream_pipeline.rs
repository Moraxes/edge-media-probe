//! Asynchroniczny potok analizy ciągłości ramek RTP i wykrywania anomalii SLA (`Stream`).

use std::time::Duration;
//
use bytes::Bytes;
use futures::{Stream, TryStreamExt};
use tokio::sync::mpsc;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::errors::BroadcastStreamRecvError;
use tokio_util::sync::CancellationToken;
//
use crate::control_plane::SlaAlertPayload;

/// Zadanie 7: zbuduj potok analizy ciągłości pakietów RTP w oparciu o kombinatory `Stream`
/// (`take_until`, `timeout`, normalizacja `Elapsed`/`BroadcastStreamRecvError`, `try_filter_map`
/// z `rtp_parser::parse_rtp_header`, `scan`, `chunks_timeout`) i publikuj wykryte luki
/// sekwencyjne jako `SlaAlertPayload` do kanału `alerts_tx`.
pub async fn run_sla_analyzer<S>(
  ssrc: u32,
  frames: S,
  alerts_tx: mpsc::Sender<SlaAlertPayload>,
  shutdown: CancellationToken,
) where
  S: Stream<Item = Result<Bytes, BroadcastStreamRecvError>> + Unpin,
{
  todo!()
}
