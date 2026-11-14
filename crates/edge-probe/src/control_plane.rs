//! Serwer `control-plane` zarządzający strumieniami oraz alertami SLA.

use std::sync::Arc;
//
use axum::Router;
use sqlx::SqlitePool;
use tokio::sync::{mpsc, oneshot, watch};

pub struct StreamInfo {}

pub struct SlaAlertPayload {}

pub struct AlertStats {}

pub type AlertCommand = (SlaAlertPayload, oneshot::Sender<Result<(), sqlx::Error>>);

pub struct AppState {}

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
    _db: SqlitePool,
    _rx: mpsc::Receiver<AlertCommand>,
    _stats_tx: watch::Sender<AlertStats>,
) {
    todo!()
}

pub fn create_router(_state: Arc<AppState>) -> Router {
    // GET /v1/streams - lista aktywnych strumieni
    // POST /v1/streams - rejestracja aktywnego strumienia
    // POST /v1/alerts - zapis alertu SLA
    // GET /v1/stats - bieżące statystyki alertów
    Router::new()
}
