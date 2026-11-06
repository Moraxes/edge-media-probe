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
        // Zadanie 1.3: Zaimplementuj mapowanie błędu domenowego na odpowiedź HTTP Axum:
        // Wymagania:
        // - Dla `ApiError::Database`: zwróć StatusCode::INTERNAL_SERVER_ERROR oraz JSON {"error": ...}
        // - Dla `ApiError::InvalidSsrc`: zwróć StatusCode::BAD_REQUEST oraz JSON {"error": ...}
        todo!("Zadanie 1.3: Zaimplementuj IntoResponse dla ApiError")
    }
}

/// Endpoint KMS: GET /v1/keys/{ssrc}
pub async fn handle_get_kms_keys(
    Path(_ssrc_hex): Path<String>,
) -> Result<Json<crate::kms_client::KmsKeyResponse>, ApiError> {
    // Zadanie 1.3: Zaimplementuj obsługę zapytania GET po materiał klucza SRTP:
    // Wymagania:
    // 1. Sparsuj ciąg hex SSRC (obsłuż opcjonalny prefix "0x" lub "0X") do wartości u32.
    // 2. W razie błędu parsowania zwróć `ApiError::InvalidSsrc`.
    // 3. Wygeneruj deterministyczny 16-bajtowy klucz AES i 14-bajtową sól (lub pobierz z repozytorium kluczy).
    // 4. Zwróć `Json(KmsKeyResponse)`.
    todo!("Zadanie 1.3: Zaimplementuj handle_get_kms_keys")
}

/// Endpoint Webhooka SLA: POST /v1/alerts
pub async fn handle_post_sla_alert(
    State(_state): State<Arc<AppState>>,
    Json(_payload): Json<crate::kms_client::SlaAlertPayload>,
) -> Result<StatusCode, ApiError> {
    // Zadanie 1.3: Zaimplementuj rejestrację alertu SLA w transakcji SQLite:
    // Wymagania:
    // 1. Rozpocznij asynchroniczną transakcję na bazie `state.db.begin().await?`.
    // 2. Wstaw wpis do tabeli `probe_alerts` (ssrc, fraction_lost, reason).
    // 3. Zatwierdź transakcję `tx.commit().await?`.
    // 4. Zwróć kod `StatusCode::CREATED`.
    todo!("Zadanie 1.3: Zaimplementuj handle_post_sla_alert z transakcją SQLx")
}

pub fn create_control_plane_router(_state: Arc<AppState>) -> Router {
    // Zadanie 1.3: Zbuduj router Axum:
    // Wymagania:
    // 1. Zarejestruj trasę GET `/v1/keys/{ssrc}` kierującą do `handle_get_kms_keys`.
    // 2. Zarejestruj trasę POST `/v1/alerts` kierującą do `handle_post_sla_alert`.
    // 3. Dołącz stan `state` za pomocą `.with_state(state)`.
    todo!("Zadanie 1.3: Zaimplementuj create_control_plane_router")
}
