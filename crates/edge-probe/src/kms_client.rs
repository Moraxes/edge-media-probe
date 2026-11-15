//! Klient KMS (Key Management Service) dla węzła `edge_probe`.

use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;

/// Zadanie 2: zdefiniuj pola struktury na podstawie odpowiedzi serwera KMS (`curl`).
#[derive(Debug, Deserialize)]
pub struct KmsKeyResponse {
  // Dodaj pola odpowiadające JSON-owi z GET /v1/keys/{ssrc}
}

/// Zadanie 1: pobierz równolegle klucze jako surowy tekst JSON dla podanych SSRC.
pub async fn fetch_keys_raw(
  _base_url: &str,
  _ssrcs: Vec<u32>,
) -> Result<(), reqwest::Error> {
  todo!()
}

/// Zadanie 2: pobierz i zdeserializuj odpowiedzi KMS do `HashMap<u32, KmsKeyResponse>`.
pub async fn fetch_keys_batch(
  _base_url: &str,
  _ssrcs: Vec<u32>,
) -> Result<HashMap<u32, KmsKeyResponse>, reqwest::Error> {
  todo!()
}

/// Zadanie 6: pobierz pojedynczy klucz SRTP dla odkrytego SSRC z `GET {base_url}/v1/keys/{ssrc:#010x}`.
pub async fn fetch_key(
  _base_url: &str,
  _ssrc: u32,
) -> Result<KmsKeyResponse, reqwest::Error> {
  todo!()
}
