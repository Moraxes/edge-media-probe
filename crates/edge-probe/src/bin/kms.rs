//! Serwer KMS (Key Management Service) dla kluczy SRTP.
//! Zadanie 4.1: dodaj stan `RwLock` oraz obsługę `POST /v1/keys/{ssrc}` w serwerze `kms`.

use axum::extract::Path;
use axum::http::StatusCode;
use axum::{Router, routing::get};
use edge_probe::parse_ssrc;
use tokio::net::TcpListener;

async fn handle_get_key(
  Path(raw_ssrc): Path<String>,
) -> Result<String, StatusCode> {
  let ssrc = parse_ssrc(&raw_ssrc).ok_or(StatusCode::BAD_REQUEST)?;
  Ok(format!(
    r#"{{"ssrc":{ssrc},"master_key_hex":"00112233445566778899aabbccddeeff","master_salt_hex":"0102030405060708090a0b0c0d0e","expires_at":"2026-12-31T23:59:59Z"}}"#
  ))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let app = Router::new().route("/v1/keys/{ssrc}", get(handle_get_key));
  let listener = TcpListener::bind("127.0.0.1:8081").await?;
  println!("kms nasłuchuje na http://127.0.0.1:8081");
  axum::serve(listener, app).await?;
  Ok(())
}
