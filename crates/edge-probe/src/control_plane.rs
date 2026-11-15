//! Serwer `control-plane` zarządzający strumieniami oraz alertami SLA.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
//
use axum::{
  Json, Router,
  extract::State,
  http::StatusCode,
  routing::{get, post},
};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use tokio::sync::{mpsc, oneshot, watch};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamInfo {
  pub ssrc: u32,
  pub label: String,
  #[serde(default)]
  pub egress_addr: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlaAlertPayload {
  pub ssrc: u32,
  pub fraction_lost: f32,
  pub reason: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AlertStats {
  pub total_alerts: u64,
  pub last_ssrc: Option<u32>,
}

pub type AlertCommand =
  (SlaAlertPayload, oneshot::Sender<Result<(), sqlx::Error>>);

pub struct AppState {
  pub streams: Mutex<HashMap<u32, StreamInfo>>,
  pub alerts_tx: mpsc::Sender<AlertCommand>,
  pub stats_rx: watch::Receiver<AlertStats>,
}

pub async fn init_db(db_url: &str) -> Result<SqlitePool, sqlx::Error> {
  let pool = SqlitePool::connect(db_url).await?;
  sqlx::raw_sql(include_str!("../schema.sql"))
    .execute(&pool)
    .await?;
  Ok(pool)
}

/// Zadanie 5b: odbieraj komendy z `rx`, wykonuj `INSERT`, odsyłaj wynik przez `oneshot`
/// i aktualizuj statystyki w `stats_tx`.
pub fn spawn_alert_writer(
  db: SqlitePool,
  mut rx: mpsc::Receiver<AlertCommand>,
  stats_tx: watch::Sender<AlertStats>,
) {
  tokio::spawn(async move {
    let mut stats = AlertStats::default();
    while let Some((payload, reply)) = rx.recv().await {
      let ssrc = payload.ssrc;
      let res = sqlx::query(
                "INSERT INTO probe_alerts (ssrc, fraction_lost, reason) VALUES (?, ?, ?)",
            )
            .bind(ssrc as i64)
            .bind(payload.fraction_lost)
            .bind(&payload.reason)
            .execute(&db)
            .await
            .map(|_| ());

      if res.is_ok() {
        stats.total_alerts += 1;
        stats.last_ssrc = Some(ssrc);
        let _ = stats_tx.send(stats.clone());
      }
      let _ = reply.send(res);
    }
  });
}

async fn handle_list_streams(
  State(state): State<Arc<AppState>>,
) -> Json<Vec<StreamInfo>> {
  let streams = state.streams.lock().unwrap();
  let mut list: Vec<StreamInfo> = streams.values().cloned().collect();
  list.sort_by_key(|s| s.ssrc);
  Json(list)
}

async fn handle_register_stream(
  State(state): State<Arc<AppState>>,
  Json(info): Json<StreamInfo>,
) -> StatusCode {
  let mut streams = state.streams.lock().unwrap();
  streams.insert(info.ssrc, info);
  StatusCode::CREATED
}

async fn handle_post_alert(
  State(state): State<Arc<AppState>>,
  Json(payload): Json<SlaAlertPayload>,
) -> Result<StatusCode, StatusCode> {
  let (reply_tx, reply_rx) = oneshot::channel();
  state
    .alerts_tx
    .send((payload, reply_tx))
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
  reply_rx
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
  Ok(StatusCode::CREATED)
}

async fn handle_get_stats(
  State(state): State<Arc<AppState>>,
) -> Json<AlertStats> {
  let stats = state.stats_rx.borrow().clone();
  Json(stats)
}

pub fn create_router(state: Arc<AppState>) -> Router {
  Router::new()
    .route(
      "/v1/streams",
      get(handle_list_streams).post(handle_register_stream),
    )
    .route("/v1/alerts", post(handle_post_alert))
    .route("/v1/stats", get(handle_get_stats))
    .with_state(state)
}
