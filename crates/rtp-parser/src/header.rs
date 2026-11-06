use crate::error::RtpParseError;
use bytes::Buf;

pub const MIN_RTP_HEADER_LEN: usize = 12;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RtpHeader {
    pub version: u8,
    pub padding: bool,
    pub extension: bool,
    pub marker: bool,
    pub payload_type: u8,
    pub sequence_number: u16,
    pub timestamp: u32,
    pub ssrc: u32,
    pub csrc: Vec<u32>,
}

impl RtpHeader {
    /// Parsuje nagłówek RTP ze strumienia bajtów `bytes::Buf`.
    /// Zwraca sparsowany nagłówek oraz ilość odczytanych bajtów nagłówka.
    pub fn parse<B: Buf>(buf: &mut B) -> Result<Self, RtpParseError> {
        let remaining = buf.remaining();
        if remaining < MIN_RTP_HEADER_LEN {
            return Err(RtpParseError::PacketTooShort {
                needed: MIN_RTP_HEADER_LEN,
                got: remaining,
            });
        }

        let b0 = buf.get_u8();
        let b1 = buf.get_u8();

        let version = b0 >> 6;
        if version != 2 {
            return Err(RtpParseError::InvalidVersion(version));
        }

        let padding = (b0 & 0x20) != 0;
        let extension = (b0 & 0x10) != 0;
        let csrc_count = (b0 & 0x0F) as usize;

        let marker = (b1 & 0x80) != 0;
        let payload_type = b1 & 0x7F;

        let sequence_number = buf.get_u16();
        let timestamp = buf.get_u32();
        let ssrc = buf.get_u32();

        let csrc_bytes = csrc_count * 4;
        if buf.remaining() < csrc_bytes {
            return Err(RtpParseError::TruncatedCsrc {
                needed: csrc_bytes,
                got: buf.remaining(),
            });
        }

        let mut csrc = Vec::with_capacity(csrc_count);
        for _ in 0..csrc_count {
            csrc.push(buf.get_u32());
        }

        Ok(Self {
            version,
            padding,
            extension,
            marker,
            payload_type,
            sequence_number,
            timestamp,
            ssrc,
            csrc,
        })
    }

    /// Oblicza długość nagłówka RTP w bajtach (wraz z identyfikatorami CSRC).
    #[inline]
    pub fn header_len(&self) -> usize {
        MIN_RTP_HEADER_LEN + (self.csrc.len() * 4)
    }
}

