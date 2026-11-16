//! Serwer KMS (Key Management Service) dla kluczy SRTP.
//! Zadanie 4.1: dodaj stan `RwLock` oraz obsługę `POST /v1/keys/{ssrc}` w serwerze `kms`.

use axum::extract::Path;
use axum::http::StatusCode;
use axum::{Json, Router, routing::get};
use chrono::{Duration, Utc};
use edge_probe::kms_client::KmsKeyResponse;
use edge_probe::parse_ssrc;
use tokio::net::TcpListener;

fn make_key(ssrc: u32) -> KmsKeyResponse {
  KmsKeyResponse {
    ssrc,
    master_key_hex: "00112233445566778899aabbccddeeff".into(),
    master_salt_hex: "00112233445566778899aabbccdd".into(),
    expires_at: Utc::now() + Duration::seconds(30 + ((ssrc % 4) * 15) as i64),
  }
}

async fn handle_get_key(
  Path(raw_ssrc): Path<String>,
) -> Result<Json<KmsKeyResponse>, StatusCode> {
  let ssrc = parse_ssrc(&raw_ssrc).ok_or(StatusCode::BAD_REQUEST)?;
  Ok(Json(make_key(ssrc)))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let app = Router::new().route("/v1/keys/{ssrc}", get(handle_get_key));
  let listener = TcpListener::bind("127.0.0.1:8081").await?;
  println!("kms nasłuchuje na http://127.0.0.1:8081");
  axum::serve(listener, app).await?;
  Ok(())
}
