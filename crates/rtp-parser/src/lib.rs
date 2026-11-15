//! Kratka pomocnicza do parsowania nagłówków RTP (RFC 3550) i demultipleksacji UDP (RFC 5761).
//!
//! Zapewnia gotowe struktury nagłówków, funkcje sprawdzające oraz mechanizmy ekstrakcji
//! ładunku bez konieczności ręcznego kodowania masek bitowych i offsetów pakietu przez uczestników.

pub mod demux;
pub mod error;
pub mod header;
pub mod packet;

pub use demux::{PacketClass, classify_peeked_packet};
pub use error::RtpParseError;
pub use header::{MIN_RTP_HEADER_LEN, RtpHeader};
pub use packet::{RtpPacket, build_test_rtp_packet, parse_rtp_header};
