//! Moduł 3 (Zadanie 3.3): Szyfrowanie SRTP (RFC 3711) i symulacja awarii w `turmoil`.
//!
//! Ćwiczenie obejmuje:
//! - Szyfrowanie ładunku AES-128-CTR przy jawnym nagłówku dla SFU,
//! - Weryfikację 10-bajtowego tagu uwierzytelnienia HMAC-SHA1 i okno Replay Protection,
//! - Deterministyczne testowanie podziału sieci (`partition_oneway`) w symulatorze `turmoil`.

use webrtc_srtp::context::Context as SrtpContext;
use webrtc_srtp::protection_profile::ProtectionProfile;

pub fn create_srtp_pair() -> anyhow::Result<(SrtpContext, SrtpContext)> {
    // Zadanie 3.3: Inicjalizacja pary kontekstów SRTP TX i RX
    todo!()
}
