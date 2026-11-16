//! Klient HTTP dla usługi KMS (Key Management Service) ze współdzieloną pulą połączeń.

use std::collections::HashMap;
//
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::task::JoinSet;

/// Odpowiedź KMS z materiałem klucza SRTP dla danego SSRC.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KmsKeyResponse {
  pub ssrc: u32,
  pub master_key_hex: String,
  pub master_salt_hex: String,
  pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct KmsClient {
  client: Client,
  base_url: String,
}

impl KmsClient {
  pub fn new(base_url: &str) -> Self {
    Self {
      client: Client::new(),
      base_url: base_url
        .trim_end_matches('/')
        .to_string(),
    }
  }

  /// Pobierz pojedynczy klucz SRTP dla odkrytego SSRC z `GET {base_url}/v1/keys/{ssrc:#010x}`.
  pub async fn fetch_key(
    &self,
    ssrc: u32,
  ) -> Result<KmsKeyResponse, reqwest::Error> {
    let url = format!("{}/v1/keys/{ssrc:#010x}", self.base_url);
    self
      .client
      .get(url)
      .send()
      .await?
      .error_for_status()?
      .json::<KmsKeyResponse>()
      .await
  }

  /// Pobierz równolegle i zdeserializuj klucze KMS dla podanych SSRC.
  pub async fn fetch_keys_batch(
    &self,
    ssrcs: Vec<u32>,
  ) -> Result<HashMap<u32, KmsKeyResponse>, reqwest::Error> {
    let mut set = JoinSet::new();
    for ssrc in ssrcs {
      let client = self.clone();
      set.spawn(async move {
        let key = client.fetch_key(ssrc).await?;
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
}
