//! Moduł 3 (Zadanie 3.3): Szyfrowanie SRTP (RFC 3711) i symulacja awarii w `turmoil`.
//!
//! Ćwiczenie obejmuje:
//! - Szyfrowanie ładunku AES-128-CTR przy jawnym nagłówku dla SFU,
//! - Weryfikację 10-bajtowego tagu uwierzytelnienia HMAC-SHA1 i okno Replay Protection,
//! - Deterministyczne testowanie podziału sieci (`partition_oneway`) w symulatorze `turmoil`.

use webrtc_srtp::context::Context as SrtpContext;
use webrtc_srtp::protection_profile::ProtectionProfile;

pub fn create_srtp_pair() -> anyhow::Result<(SrtpContext, SrtpContext)> {
    let master_key = [0x11u8; 16];
    let master_salt = [0x22u8; 14];
    let profile = ProtectionProfile::Aes128CmHmacSha1_80;

    let tx_ctx = SrtpContext::new(&master_key, &master_salt, profile, None, None)?;
    let rx_ctx = SrtpContext::new(&master_key, &master_salt, profile, None, None)?;
    Ok((tx_ctx, rx_ctx))
}


