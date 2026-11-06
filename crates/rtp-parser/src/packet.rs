use crate::error::RtpParseError;
use crate::header::RtpHeader;
use bytes::{Buf, BufMut, Bytes, BytesMut};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RtpPacket {
    pub header: RtpHeader,
    pub payload: Bytes,
}

impl RtpPacket {
    pub fn parse(raw: Bytes) -> Result<Self, RtpParseError> {
        let (header, payload) = parse_rtp_header(raw)?;
        Ok(Self { header, payload })
    }
}

/// Parsuje nagłówek pakietu RTP i zwraca krotkę `(RtpHeader, Bytes)` zawierającą nagłówek
/// oraz pozostały ładunek pakietu (payload).
///
/// Obsługuje wersję protokołu (RFC 3550), nagłówki CSRC, rozszerzenia nagłówka (Extension Header)
/// oraz odcinanie bajtów dopełnienia (Padding).
pub fn parse_rtp_header(mut packet: Bytes) -> Result<(RtpHeader, Bytes), RtpParseError> {
    let header = RtpHeader::parse(&mut packet)?;

    // Obsługa rozszerzenia nagłówka (RFC 3550 sekcja 5.3.1)
    if header.extension {
        if packet.remaining() < 4 {
            return Err(RtpParseError::TruncatedExtension);
        }
        let _profile = packet.get_u16();
        let ext_words = packet.get_u16() as usize;
        let ext_bytes = ext_words * 4;
        if packet.remaining() < ext_bytes {
            return Err(RtpParseError::TruncatedExtension);
        }
        packet.advance(ext_bytes);
    }

    // Obsługa dopełnienia (Padding)
    if header.padding {
        let remaining_len = packet.len();
        if remaining_len == 0 {
            return Err(RtpParseError::InvalidPadding {
                padding_len: 0,
                payload_len: 0,
            });
        }
        let pad_len = packet[remaining_len - 1] as usize;
        if pad_len == 0 || pad_len > remaining_len {
            return Err(RtpParseError::InvalidPadding {
                padding_len: pad_len,
                payload_len: remaining_len,
            });
        }
        packet.truncate(remaining_len - pad_len);
    }

    Ok((header, packet))
}

/// Buduje przykładowy pakiet RTP w celach testowych lub laboratoryjnych.
pub fn build_test_rtp_packet(seq: u16, ts: u32, ssrc: u32, payload: &[u8]) -> Bytes {
    let mut buf = BytesMut::with_capacity(12 + payload.len());
    buf.put_u8(0x80); // V=2, P=0, X=0, CC=0
    buf.put_u8(96);   // M=0, PT=96 (dynamiczny H.264/Opus)
    buf.put_u16(seq);
    buf.put_u32(ts);
    buf.put_u32(ssrc);
    buf.put_slice(payload);
    buf.freeze()
}
