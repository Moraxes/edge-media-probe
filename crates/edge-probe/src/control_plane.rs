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
  pub db: SqlitePool,
  pub streams: Mutex<HashMap<u32, StreamInfo>>,
}

pub async fn init_db(db_url: &str) -> Result<SqlitePool, sqlx::Error> {
  let pool = SqlitePool::connect(db_url).await?;
  sqlx::raw_sql(include_str!("../schema.sql"))
    .execute(&pool)
    .await?;
  Ok(pool)
}

/// Zadanie 5.1: odbieraj alerty z `rx` i zapisuj je sekwencyjnie do tabeli `probe_alerts`.
/// Zadanie 5.2: odbieraj komendy z `rx`, wykonuj `INSERT`, odsyłaj wynik przez `oneshot`
/// i aktualizuj statystyki w `stats_tx`.
pub fn spawn_alert_writer(
  _db: SqlitePool,
  _rx: mpsc::Receiver<AlertCommand>,
  _stats_tx: watch::Sender<AlertStats>,
) {
  todo!()
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
  sqlx::query(
    "INSERT INTO probe_alerts (ssrc, fraction_lost, reason) VALUES (?, ?, ?)",
  )
  .bind(payload.ssrc as i64)
  .bind(payload.fraction_lost)
  .bind(&payload.reason)
  .execute(&state.db)
  .await
  .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
  Ok(StatusCode::CREATED)
}

pub fn create_router(state: Arc<AppState>) -> Router {
  // GET /v1/streams - lista aktywnych strumieni
  // POST /v1/streams - rejestracja aktywnego strumienia
  // POST /v1/alerts - zapis alertu SLA
  // GET /v1/stats - bieżące statystyki alertów
  Router::new()
    .route(
      "/v1/streams",
      get(handle_list_streams).post(handle_register_stream),
    )
    .route("/v1/alerts", post(handle_post_alert))
    .with_state(state)
}
