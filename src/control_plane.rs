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
    routing::{get, post},
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
        let (status, msg) = match self {
            ApiError::Database(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            ApiError::InvalidSsrc(msg) => (StatusCode::BAD_REQUEST, msg),
        };
        (status, Json(serde_json::json!({ "error": msg }))).into_response()
    }
}

/// Endpoint KMS: GET /v1/keys/{ssrc}
pub async fn handle_get_kms_keys(
    Path(ssrc_hex): Path<String>,
) -> Result<Json<crate::kms_client::KmsKeyResponse>, ApiError> {
    let ssrc_trimmed = ssrc_hex.trim_start_matches("0x").trim_start_matches("0X");
    let ssrc_val = u32::from_str_radix(ssrc_trimmed, 16)
        .map_err(|_| ApiError::InvalidSsrc(format!("Niepoprawna wartość hex: {ssrc_hex}")))?;

    // Deterministyczny materiał klucza SRTP dla podanego SSRC (AES-128 + sól)
    let key_bytes: Vec<u8> = (0..16).map(|i| ((ssrc_val >> ((i % 4) * 8)) & 0xFF) as u8 ^ i).collect();
    let salt_bytes: Vec<u8> = (0..14).map(|i| (i * 7 + 3) as u8).collect();

    Ok(Json(crate::kms_client::KmsKeyResponse {
        ssrc: ssrc_val,
        master_key_hex: hex::encode(key_bytes),
        master_salt_hex: hex::encode(salt_bytes),
    }))
}

/// Endpoint Webhooka SLA: POST /v1/alerts
pub async fn handle_post_sla_alert(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<crate::kms_client::SlaAlertPayload>,
) -> Result<StatusCode, ApiError> {
    let mut tx = state.db.begin().await?;

    sqlx::query(
        "INSERT INTO probe_alerts (ssrc, fraction_lost, reason) VALUES (?, ?, ?)",
    )
    .bind(payload.ssrc as i64)
    .bind(payload.fraction_lost)
    .bind(&payload.reason)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(StatusCode::CREATED)
}

pub fn create_control_plane_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/v1/keys/{ssrc}", get(handle_get_kms_keys))
        .route("/v1/alerts", post(handle_post_sla_alert))
        .with_state(state)
}


