//! Weryfikacja ważności klucza KMS oraz integralności pakietów SRTP (`webrtc-srtp`).

use chrono::{DateTime, Utc};
use thiserror::Error;
use webrtc_srtp::context::Context;
use webrtc_srtp::option::{srtcp_replay_protection, srtp_replay_protection};
use webrtc_srtp::protection_profile::ProtectionProfile;
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
  pub fn from_kms_key(key: &KmsKeyResponse) -> Result<Self, SrtpLayerError> {
    let master_key = hex::decode(&key.master_key_hex)?;
    let master_salt = hex::decode(&key.master_salt_hex)?;
    let ctx = Context::new(
      &master_key,
      &master_salt,
      ProtectionProfile::Aes128CmHmacSha1_80,
      Some(srtp_replay_protection(64)),
      Some(srtcp_replay_protection(64)),
    )?;
    Ok(Self {
      ctx,
      expires_at: key.expires_at,
    })
  }

  fn check_expiry_and_copy(
    &self,
    raw: &[u8],
  ) -> Result<Vec<u8>, SrtpLayerError> {
    if self.expires_at <= Utc::now() {
      return Err(SrtpLayerError::KeyExpired);
    }
    Ok(raw.to_vec())
  }

  /// Weryfikuje ważność klucza oraz podpis HMAC i okno anty-powtórzeniowe
  /// na kopii roboczej, pozostawiając oryginalny bufor SRTP nienaruszony.
  pub fn verify_packet(
    &mut self,
    raw_srtp: &[u8],
  ) -> Result<(), SrtpLayerError> {
    let mut scratch = self.check_expiry_and_copy(raw_srtp)?;
    self.ctx.decrypt_rtp(&mut scratch)?;
    Ok(())
  }

  /// Weryfikuje ważność klucza oraz podpis HMAC i okno anty-powtórzeniowe
  /// na kopii roboczej, pozostawiając oryginalny bufor SRTCP nienaruszony.
  pub fn verify_rtcp_packet(
    &mut self,
    raw_srtcp: &[u8],
  ) -> Result<(), SrtpLayerError> {
    let mut scratch = self.check_expiry_and_copy(raw_srtcp)?;
    self.ctx.decrypt_rtcp(&mut scratch)?;
    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use chrono::Duration as ChronoDuration;
  use rtp_parser::build_test_rtp_packet;

  fn test_kms_key(expires_at: DateTime<Utc>) -> KmsKeyResponse {
    KmsKeyResponse {
      ssrc: 0x1000,
      master_key_hex: "000102030405060708090a0b0c0d0e0f".into(),
      master_salt_hex: "0e0d0c0b0a090807060504030201".into(),
      expires_at,
    }
  }

  fn encrypt_sample(seq: u16) -> Vec<u8> {
    let key = hex::decode("000102030405060708090a0b0c0d0e0f").unwrap();
    let salt = hex::decode("0e0d0c0b0a090807060504030201").unwrap();
    let mut tx = Context::new(
      &key,
      &salt,
      ProtectionProfile::Aes128CmHmacSha1_80,
      None,
      None,
    )
    .unwrap();
    let rtp = build_test_rtp_packet(seq, 90_000, 0x1000, b"video-frame");
    tx.encrypt_rtp(&rtp).unwrap().to_vec()
  }

  #[test]
  fn srtp_layer_verifies_hmac_replay_and_expiry() {
    let key = test_kms_key(Utc::now() + ChronoDuration::minutes(5));
    let mut verifier = SrtpVerifier::from_kms_key(&key).unwrap();

    let pkt1 = encrypt_sample(1);
    assert!(verifier.verify_packet(&pkt1).is_ok());

    // Ponowna weryfikacja tego samego pakietu uruchamia Replay Protection.
    assert!(matches!(
      verifier.verify_packet(&pkt1),
      Err(SrtpLayerError::Srtp(_))
    ));

    // Modyfikacja jawnego numeru sekwencyjnego psuje HMAC-SHA1.
    let mut tampered = encrypt_sample(2);
    tampered[3] ^= 0x01;
    assert!(matches!(
      verifier.verify_packet(&tampered),
      Err(SrtpLayerError::Srtp(_))
    ));

    let expired_key = test_kms_key(Utc::now() - ChronoDuration::seconds(1));
    let mut expired = SrtpVerifier::from_kms_key(&expired_key).unwrap();
    let pkt3 = encrypt_sample(3);
    assert!(matches!(
      expired.verify_packet(&pkt3),
      Err(SrtpLayerError::KeyExpired)
    ));
  }

  fn encrypt_rtcp_sample() -> Vec<u8> {
    let key = hex::decode("000102030405060708090a0b0c0d0e0f").unwrap();
    let salt = hex::decode("0e0d0c0b0a090807060504030201").unwrap();
    let mut tx = Context::new(
      &key,
      &salt,
      ProtectionProfile::Aes128CmHmacSha1_80,
      None,
      None,
    )
    .unwrap();
    let rtcp = [0x80u8, 201, 0x00, 0x01, 0x00, 0x00, 0x10, 0x00];
    tx.encrypt_rtcp(&rtcp).unwrap().to_vec()
  }

  #[test]
  fn srtp_layer_verifies_srtcp_hmac_replay_and_expiry() {
    let key = test_kms_key(Utc::now() + ChronoDuration::minutes(5));
    let mut verifier = SrtpVerifier::from_kms_key(&key).unwrap();

    let srtcp1 = encrypt_rtcp_sample();
    assert!(
      verifier
        .verify_rtcp_packet(&srtcp1)
        .is_ok()
    );

    // Ponowna weryfikacja tego samego pakietu uruchamia SRTCP Replay Protection.
    assert!(matches!(
      verifier.verify_rtcp_packet(&srtcp1),
      Err(SrtpLayerError::Srtp(_))
    ));

    let mut tampered = encrypt_rtcp_sample();
    tampered[2] ^= 0x01;
    assert!(matches!(
      verifier.verify_rtcp_packet(&tampered),
      Err(SrtpLayerError::Srtp(_))
    ));

    let expired_key = test_kms_key(Utc::now() - ChronoDuration::seconds(1));
    let mut expired = SrtpVerifier::from_kms_key(&expired_key).unwrap();
    let srtcp2 = encrypt_rtcp_sample();
    assert!(matches!(
      expired.verify_rtcp_packet(&srtcp2),
      Err(SrtpLayerError::KeyExpired)
    ));
  }
}
