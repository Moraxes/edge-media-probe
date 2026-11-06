//! Moduł 3 (Zadanie 3.3): Szyfrowanie SRTP (RFC 3711) i symulacja awarii w `turmoil`.
//!
//! Ćwiczenie obejmuje:
//! - Szyfrowanie ładunku AES-128-CTR przy jawnym nagłówku dla SFU,
//! - Weryfikację 10-bajtowego tagu uwierzytelnienia HMAC-SHA1 i okno Replay Protection,
//! - Deterministyczne testowanie podziału sieci (`partition_oneway`) w symulatorze `turmoil`.

use webrtc_srtp::context::Context as SrtpContext;
use webrtc_srtp::protection_profile::ProtectionProfile;

pub fn create_srtp_pair() -> anyhow::Result<(SrtpContext, SrtpContext)> {
    // Zadanie 3.3: Skonfiguruj parę kontekstów SRTP (nadajnik TX i odbiornik RX):
    // Wymagania:
    // 1. Zdefiniuj 16-bajtowy klucz główny (np. [0x11; 16]) oraz 14-bajtową sól (np. [0x22; 14]).
    // 2. Wybierz profil ochrony `ProtectionProfile::Aes128CmHmacSha1_80`.
    // 3. Zainicjalizuj `tx_ctx` i `rx_ctx` za pomocą `SrtpContext::new(...)`.
    // 4. Zwróć parę `(tx_ctx, rx_ctx)`.
    todo!("Zadanie 3.3: Zaimplementuj create_srtp_pair z webrtc-srtp")
}
