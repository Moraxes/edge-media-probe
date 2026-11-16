//! Weryfikacja ważności klucza KMS oraz integralności pakietów SRTP (`webrtc-srtp`).

use chrono::{DateTime, Utc};
use thiserror::Error;
use webrtc_srtp::context::Context;
//
use crate::kms_client::KmsKeyResponse;

#[derive(Debug, Error)]
pub enum SrtpLayerError {
    #[error("klucz SRTP wygasł")]
    KeyExpired,
    #[error("błąd dekodowania hex klucza/soli: {0}")]
    InvalidKeyMaterial(#[from] hex::FromHexError),
    #[error("błąd weryfikacji SRTP: {0}")]
    Srtp(#[from] webrtc_srtp::Error),
}

pub struct SrtpVerifier {
    pub ctx: Context,
    pub expires_at: DateTime<Utc>,
}

impl SrtpVerifier {
    /// Zadanie 10: zdekoduj `master_key_hex` i `master_salt_hex` kratką `hex`,
    /// a następnie utwórz `Context` z profilem `ProtectionProfile::Aes128CmHmacSha1_80`
    /// oraz ochroną przed powtórzeniami (`srtp_replay_protection(64)`, `srtcp_replay_protection(64)`).
    pub fn from_kms_key(key: &KmsKeyResponse) -> Result<Self, SrtpLayerError> {
        todo!()
    }

    /// Zadanie 10: sprawdź `expires_at > Utc::now()` i zweryfikuj pakiet RTP przez `decrypt_rtp`
    /// na kopii roboczej, aby oryginalny bufor SRTP pozostał nienaruszony.
    pub fn verify_packet(&mut self, raw_srtp: &[u8]) -> Result<(), SrtpLayerError> {
        todo!()
    }

    /// Zadanie 10: sprawdź `expires_at > Utc::now()` i zweryfikuj pakiet RTCP przez `decrypt_rtcp`
    /// na kopii roboczej, aby oryginalny bufor SRTCP pozostał nienaruszony.
    pub fn verify_rtcp_packet(&mut self, raw_srtcp: &[u8]) -> Result<(), SrtpLayerError> {
        todo!()
    }
}
