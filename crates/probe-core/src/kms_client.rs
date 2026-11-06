//! Moduł 1 (Zadanie 1.2): Klient KMS i webhooków alarmowych w `reqwest` + `rustls`.
//!
//! Ćwiczenie obejmuje:
//! - Konfigurację czystego klienta TLS w Ruście (`reqwest::ClientBuilder::use_rustls_tls`),
//! - Zarządzanie pulą połączeń (Connection Pooling) i maskowanie sekretów w `Debug`,
//! - Pobieranie materiału klucza SRTP (`fetch_srtp_keys`) i wysyłanie alertów SLA (`send_sla_alert`).

use serde::{Deserialize, Serialize};
use std::fmt;

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
    pub fn new(_base_url: String) -> Result<Self, reqwest::Error> {
        // Zadanie 1.2: Skonfiguruj klienta reqwest z obsługą rustls
        todo!()
    }

    /// Pobiera materiał klucza SRTP dla strumienia o zadanym SSRC
    pub async fn fetch_srtp_keys(&self, _ssrc: u32) -> Result<SrtpKeyMaterial, KmsError> {
        // Zadanie 1.2: Pobierz materiał klucza SRTP z KMS przez GET
        todo!()
    }

    /// Wysyła powiadomienie o naruszeniu SLA lub anomalii jakości strumienia
    pub async fn send_sla_alert(&self, _alert: &SlaAlertPayload) -> Result<(), KmsError> {
        // Zadanie 1.2: Wyślij alert SLA jako JSON przez POST
        todo!()
    }
}
