//! Serwer `control-plane` zarządzający strumieniami oraz alertami SLA.

use std::sync::Arc;
//
use axum::Router;
use sqlx::SqlitePool;
use tokio::sync::mpsc;

pub struct StreamInfo {}

pub struct SlaAlertPayload {}

pub struct AppState {}

pub async fn init_db(db_url: &str) -> Result<SqlitePool, sqlx::Error> {
    let pool = SqlitePool::connect(db_url).await?;
    sqlx::raw_sql(include_str!("../schema.sql"))
        .execute(&pool)
        .await?;
    Ok(pool)
}

/// Zadanie 5a: odbieraj alerty z `rx` i zapisuj je sekwencyjnie do tabeli `probe_alerts`.
pub fn spawn_alert_writer(_db: SqlitePool, _rx: mpsc::Receiver<SlaAlertPayload>) {
    todo!()
}

pub fn create_router(_state: Arc<AppState>) -> Router {
    // GET /v1/streams - lista aktywnych strumieni
    // POST /v1/streams - rejestracja aktywnego strumienia
    // POST /v1/alerts - zapis alertu SLA
    Router::new()
}
