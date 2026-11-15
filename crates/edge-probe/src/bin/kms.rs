//! Serwer KMS (Key Management Service) dla kluczy SRTP.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
//
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::{Json, Router, routing::get};
use chrono::{Duration, Utc};
use edge_probe::kms_client::KmsKeyResponse;
use edge_probe::parse_ssrc;
use tokio::net::TcpListener;
use tokio::signal;

pub struct KmsState {
  pub keys: RwLock<HashMap<u32, KmsKeyResponse>>,
}

fn make_key(ssrc: u32) -> KmsKeyResponse {
  let mut key = [0u8; 16];
  let mut salt = [0u8; 14];
  rand::fill(&mut key);
  rand::fill(&mut salt);
  KmsKeyResponse {
    ssrc,
    master_key_hex: hex::encode(key),
    master_salt_hex: hex::encode(salt),
    expires_at: Utc::now() + Duration::seconds(30 + ((ssrc % 4) * 15) as i64),
  }
}

async fn handle_get_key(
  State(state): State<Arc<KmsState>>,
  Path(raw_ssrc): Path<String>,
) -> Result<Json<KmsKeyResponse>, StatusCode> {
  let ssrc = parse_ssrc(&raw_ssrc).ok_or(StatusCode::BAD_REQUEST)?;
  let keys = state.keys.read().unwrap();
  keys
    .get(&ssrc)
    .cloned()
    .map(Json)
    .ok_or(StatusCode::NOT_FOUND)
}

async fn handle_create_key(
  State(state): State<Arc<KmsState>>,
  Path(raw_ssrc): Path<String>,
) -> Result<(StatusCode, Json<KmsKeyResponse>), StatusCode> {
  let ssrc = parse_ssrc(&raw_ssrc).ok_or(StatusCode::BAD_REQUEST)?;
  let key = make_key(ssrc);
  state
    .keys
    .write()
    .unwrap()
    .insert(ssrc, key.clone());
  Ok((StatusCode::CREATED, Json(key)))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let state = Arc::new(KmsState {
    keys: RwLock::new(HashMap::new()),
  });

  let app = Router::new()
    .route(
      "/v1/keys/{ssrc}",
      get(handle_get_key).post(handle_create_key),
    )
    .with_state(state);

  let listener = TcpListener::bind("127.0.0.1:8081").await?;
  println!("kms nasłuchuje na http://127.0.0.1:8081");

  axum::serve(listener, app)
    .with_graceful_shutdown(async {
      let _ = signal::ctrl_c().await;
    })
    .await?;

  Ok(())
}
