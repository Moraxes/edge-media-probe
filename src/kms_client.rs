//! Moduł 1 (Zadanie 1.2): Klient KMS i webhooków alarmowych w `reqwest` + `rustls`.
//!
//! Ćwiczenie obejmuje:
//! - Konfigurację czystego klienta TLS w Ruście (`reqwest::ClientBuilder::use_rustls_tls`),
//! - Zarządzanie pulą połączeń (Connection Pooling) i maskowanie sekretów w `Debug`,
//! - Pobieranie materiału klucza SRTP (`fetch_srtp_keys`) i wysyłanie alertów SLA (`send_sla_alert`).

use serde::{Deserialize, Serialize};
use std::fmt;
use std::time::Duration;

#[derive(Clone)]
pub struct SrtpKeyMaterial {
    pub ssrc: u32,
    pub master_key: [u8; 16],  // AES-128
    pub master_salt: [u8; 14], // 112-bitowa sól SRTP (RFC 3711)
}

impl fmt::Debug for SrtpKeyMaterial {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SrtpKeyMaterial")
            .field("ssrc", &format_args!("{:#010x}", self.ssrc))
            .field("master_key", &"[REDACTED_16_BYTES]")
            .field("master_salt", &"[REDACTED_14_BYTES]")
            .finish()
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct KmsKeyResponse {
    pub ssrc: u32,
    pub master_key_hex: String,
    pub master_salt_hex: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SlaAlertPayload {
    pub probe_id: String,
    pub ssrc: u32,
    pub fraction_lost: f32,
    pub reason: String,
}

#[derive(Debug, thiserror::Error)]
pub enum KmsError {
    #[error("Błąd sieciowy HTTP: {0}")]
    Http(#[from] reqwest::Error),
    #[error("Błąd dekodowania klucza hex: {0}")]
    Hex(#[from] hex::FromHexError),
    #[error("Nieprawidłowy rozmiar materiału klucza: oczekiwano {expected} B, otrzymano {actual} B")]
    InvalidKeyLength { expected: usize, actual: usize },
    #[error("Serwer KMS zwrócił status błędu: {0}")]
    ServerError(reqwest::StatusCode),
}

#[derive(Clone)]
pub struct KmsClient {
    client: reqwest::Client,
    base_url: String,
}

impl KmsClient {
    pub fn new(base_url: String) -> Result<Self, reqwest::Error> {
        let client = reqwest::Client::builder()
            .use_rustls_tls()
            .timeout(Duration::from_secs(5))
            .pool_idle_timeout(Duration::from_secs(90))
            .tcp_nodelay(true)
            .build()?;

        Ok(Self { client, base_url })
    }

    /// Pobiera materiał klucza SRTP dla strumienia o zadanym SSRC
    pub async fn fetch_srtp_keys(&self, ssrc: u32) -> Result<SrtpKeyMaterial, KmsError> {
        let url = format!("{}/v1/keys/{:#010x}", self.base_url, ssrc);
        let resp = self
            .client
            .get(&url)
            .header("Accept", "application/json")
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(KmsError::ServerError(resp.status()));
        }

        let body = resp.json::<KmsKeyResponse>().await?;

        let key_bytes = hex::decode(&body.master_key_hex)?;
        let salt_bytes = hex::decode(&body.master_salt_hex)?;

        if key_bytes.len() != 16 {
            return Err(KmsError::InvalidKeyLength {
                expected: 16,
                actual: key_bytes.len(),
            });
        }
        if salt_bytes.len() != 14 {
            return Err(KmsError::InvalidKeyLength {
                expected: 14,
                actual: salt_bytes.len(),
            });
        }

        let mut master_key = [0u8; 16];
        let mut master_salt = [0u8; 14];
        master_key.copy_from_slice(&key_bytes);
        master_salt.copy_from_slice(&salt_bytes);

        Ok(SrtpKeyMaterial {
            ssrc: body.ssrc,
            master_key,
            master_salt,
        })
    }

    /// Wysyła powiadomienie o naruszeniu SLA lub anomalii jakości strumienia
    pub async fn send_sla_alert(&self, alert: &SlaAlertPayload) -> Result<(), KmsError> {
        let url = format!("{}/v1/alerts", self.base_url);
        let resp = self
            .client
            .post(&url)
            .json(alert)
            .timeout(Duration::from_secs(3))
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(KmsError::ServerError(resp.status()));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{routing::get, Json, Router};

    async fn spawn_mock_kms() -> String {
        let app = Router::new().route(
            "/v1/keys/{ssrc}",
            get(|axum::extract::Path(ssrc_hex): axum::extract::Path<String>| async move {
                let ssrc_val = u32::from_str_radix(ssrc_hex.trim_start_matches("0x"), 16).unwrap_or(0);
                Json(KmsKeyResponse {
                    ssrc: ssrc_val,
                    master_key_hex: "000102030405060708090a0b0c0d0e0f".to_string(),
                    master_salt_hex: "000102030405060708090a0b0c0d".to_string(),
                })
            }),
        );

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        format!("http://{addr}")
    }

    #[tokio::test]
    async fn test_fetch_srtp_keys_from_mock_kms() {
        let server_url = spawn_mock_kms().await;
        let client = KmsClient::new(server_url).unwrap();

        let keys = client.fetch_srtp_keys(0xCAFE_BABE).await.unwrap();
        assert_eq!(keys.ssrc, 0xCAFE_BABE);
        assert_eq!(keys.master_key[0], 0x00);
        assert_eq!(keys.master_salt[0], 0x00);
    }

    #[test]
    fn test_secret_is_masked_in_debug() {
        let key = SrtpKeyMaterial {
            ssrc: 0xCAFE_BABE,
            master_key: [0xAA; 16],
            master_salt: [0xBB; 14],
        };
        let debug_str = format!("{key:?}");
        assert!(!debug_str.contains("aa"));
        assert!(debug_str.contains("[REDACTED_16_BYTES]"));
    }
}

