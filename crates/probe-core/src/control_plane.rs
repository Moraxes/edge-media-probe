//! Moduł 1 (Zadanie 1.3): Serwer Control Plane i KMS w `Axum` z `SQLx` (SQLite).
//!
//! Ćwiczenie obejmuje:
//! - Wystawienie endpointu KMS `GET /v1/keys/{ssrc}`,
//! - Zapis alertów SLA w transakcji SQLite `POST /v1/alerts`,
//! - Mapowanie błędów domenowych `ApiError` na odpowiedzi HTTP `IntoResponse`.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json, Router,
};
use sqlx::{Pool, Sqlite};
use std::sync::Arc;

pub struct AppState {
    pub db: Pool<Sqlite>,
}

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("Błąd bazy danych: {0}")]
    Database(#[from] sqlx::Error),
    #[error("Nieprawidłowy format SSRC: {0}")]
    InvalidSsrc(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        // Zadanie 1.3: Zaimplementuj mapowanie błędu domenowego na odpowiedź HTTP
        todo!()
    }
}

/// Endpoint KMS: GET /v1/keys/{ssrc}
pub async fn handle_get_kms_keys(
    Path(_ssrc_hex): Path<String>,
) -> Result<Json<crate::kms_client::KmsKeyResponse>, ApiError> {
    // Zadanie 1.3: Zwróć materiał klucza SRTP dla podanego SSRC
    todo!()
}

/// Endpoint Webhooka SLA: POST /v1/alerts
pub async fn handle_post_sla_alert(
    State(_state): State<Arc<AppState>>,
    Json(_payload): Json<crate::kms_client::SlaAlertPayload>,
) -> Result<StatusCode, ApiError> {
    // Zadanie 1.3: Zarejestruj alert SLA w transakcji SQLite
    todo!()
}

pub fn create_control_plane_router(_state: Arc<AppState>) -> Router {
    // Zadanie 1.3: Skonfiguruj router tras Axum
    todo!()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlProtocol {
    Http,
    BinaryProbe,
    Unknown,
}

/// Klasyfikuje protokół na porcie sterującym przez podgląd pierwszych 4 bajtów (`TcpStream::peek`)
/// bez ich konsumowania z kolejki odbiorczej jądra.
pub async fn peek_control_protocol(
    _stream: &tokio::net::TcpStream,
) -> std::io::Result<ControlProtocol> {
    // Zadanie 1.3: Podejrzyj pierwsze 4 bajty przez stream.peek(&mut prefix).await:
    // - b"GET " lub b"POST" -> ControlProtocol::Http
    // - b"PRBE" -> ControlProtocol::BinaryProbe
    // - pozostałe -> ControlProtocol::Unknown
    todo!()
}
