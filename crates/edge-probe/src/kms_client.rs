//! Klient KMS (Key Management Service) dla węzła `edge_probe`.

use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::task::JoinSet;

/// Zadanie 2: odpowiedź KMS z materiałem klucza SRTP dla danego SSRC.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KmsKeyResponse {
  pub ssrc: u32,
  pub master_key_hex: String,
  pub master_salt_hex: String,
  pub expires_at: DateTime<Utc>,
}

/// Zadanie 1b: pobierz równolegle klucze jako surowy tekst JSON dla podanych SSRC.
pub async fn fetch_keys_raw(
  base_url: &str,
  ssrcs: Vec<u32>,
) -> Result<(), reqwest::Error> {
  let client = Client::new();
  let base_url = base_url.trim_end_matches('/');
  let mut handles = Vec::new();
  for ssrc in ssrcs {
    let client = client.clone();
    let url = format!("{base_url}/v1/keys/{ssrc:#010x}");
    handles.push(tokio::spawn(async move {
      let body = client
        .get(url)
        .send()
        .await?
        .text()
        .await?;
      println!("{ssrc:#010x}: {body}");
      Ok::<_, reqwest::Error>(())
    }));
  }
  for handle in handles {
    if let Ok(res) = handle.await {
      res?;
    }
  }
  Ok(())
}

/// Zadanie 2: pobierz i zdeserializuj odpowiedzi KMS do `HashMap<u32, KmsKeyResponse>`.
pub async fn fetch_keys_batch(
  base_url: &str,
  ssrcs: Vec<u32>,
) -> Result<HashMap<u32, KmsKeyResponse>, reqwest::Error> {
  let client = Client::new();
  let base_url = base_url.trim_end_matches('/');
  let mut set = JoinSet::new();
  for ssrc in ssrcs {
    let client = client.clone();
    let url = format!("{base_url}/v1/keys/{ssrc:#010x}");
    set.spawn(async move {
      let key = client
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .json::<KmsKeyResponse>()
        .await?;
      Ok::<_, reqwest::Error>((ssrc, key))
    });
  }
  let mut keys = HashMap::new();
  while let Some(res) = set.join_next().await {
    if let Ok(item) = res {
      let (ssrc, key) = item?;
      keys.insert(ssrc, key);
    }
  }
  Ok(keys)
}

/// Zadanie 6: pobierz pojedynczy klucz SRTP dla odkrytego SSRC z `GET {base_url}/v1/keys/{ssrc:#010x}`.
pub async fn fetch_key(
  _base_url: &str,
  _ssrc: u32,
) -> Result<KmsKeyResponse, reqwest::Error> {
  todo!()
}
