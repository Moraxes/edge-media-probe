#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PacketClass {
    Rtp,
    Rtcp,
}

/// Dokonuje klasyfikacji pakietu UDP na podstawie pierwszych bajtów nagłówka (RFC 5761 - `rtcp-mux`).
///
/// Sprawdza wersję RTP (bity 6-7 pierwszego bajtu muszą mieć wartość 2) oraz przedział
/// typów ładunku RTCP (200..=208: SR, RR, SDES, BYE, APP, RTPFB, PSFB).
pub fn classify_peeked_packet(peek_buf: &[u8]) -> Option<PacketClass> {
    if peek_buf.len() < 2 {
        return None;
    }
    let version = peek_buf[0] >> 6;
    if version != 2 {
        return None;
    }
    if (200..=208).contains(&peek_buf[1]) {
        Some(PacketClass::Rtcp)
    } else {
        Some(PacketClass::Rtp)
    }
}

