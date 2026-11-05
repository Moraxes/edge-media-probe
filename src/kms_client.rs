//! Moduł 1 (Zadanie 1.2): Klient KMS i webhooków alarmowych w `reqwest` + `rustls`.
//!
//! Ćwiczenie obejmuje:
//! - Konfigurację czystego klienta TLS w Ruście (`reqwest::ClientBuilder::use_rustls_tls`),
//! - Zarządzanie pulą połączeń (Connection Pooling) i maskowanie sekretów w `Debug`,
//! - Pobieranie materiału klucza SRTP (`fetch_srtp_keys`) i wysyłanie alertów SLA (`send_sla_alert`).

use serde::{Deserialize, Serialize};
use std::fmt;
use std::time::Duration;
use tracing::{info, warn, Span};

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
    #[tracing::instrument(
        name = "kms.fetch_srtp_keys",
        skip(self),
        fields(
            ssrc = %format!("{ssrc:#010x}"),
            correlation_id = tracing::field::Empty,
            http.status = tracing::field::Empty
        )
    )]
    pub async fn fetch_srtp_keys(&self, ssrc: u32) -> Result<SrtpKeyMaterial, KmsError> {
        let correlation_id = format!("probe-req-{ssrc:08x}");
        Span::current().record("correlation_id", &correlation_id);

        let url = format!("{}/v1/keys/{:#010x}", self.base_url, ssrc);
        info!(url = %url, "Wysyłanie zapytania KMS o materiał klucza SRTP");

        let resp = self
            .client
            .get(&url)
            .header("X-Correlation-ID", &correlation_id)
            .header("Accept", "application/json")
            .send()
            .await?;

        Span::current().record("http.status", resp.status().as_u16());

        if !resp.status().is_success() {
            warn!(status = %resp.status(), "Serwer KMS zwrócił błąd HTTP");
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

        info!("Pomyślnie pobrano i zweryfikowano materiał kryptograficzny SRTP");

        Ok(SrtpKeyMaterial {
            ssrc: body.ssrc,
            master_key,
            master_salt,
        })
    }

    /// Wysyła powiadomienie o naruszeniu SLA lub anomalii jakości strumienia
    #[tracing::instrument(
        name = "control_plane.send_sla_alert",
        skip(self, alert),
        fields(
            ssrc = %format!("{:#010x}", alert.ssrc),
            probe_id = %alert.probe_id,
            fraction_lost = alert.fraction_lost,
            http.status = tracing::field::Empty
        )
    )]
    pub async fn send_sla_alert(&self, alert: &SlaAlertPayload) -> Result<(), KmsError> {
        let url = format!("{}/v1/alerts", self.base_url);
        info!(reason = %alert.reason, "Wysyłanie alertu SLA do Control Plane");

        let resp = self
            .client
            .post(&url)
            .json(alert)
            .timeout(Duration::from_secs(3))
            .send()
            .await?;

        Span::current().record("http.status", resp.status().as_u16());

        if !resp.status().is_success() {
            warn!(status = %resp.status(), "Błąd wysyłania alertu SLA do Control Plane");
            return Err(KmsError::ServerError(resp.status()));
        }

        info!("Alert SLA został przyjęty przez Control Plane");
        Ok(())
    }
}


