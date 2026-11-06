//! Kratka pomocnicza do parsowania nagłówków RTP (RFC 3550) i demultipleksacji UDP (RFC 5761).
//!
//! Zapewnia gotowe struktury nagłówków, funkcje sprawdzające oraz mechanizmy ekstrakcji
//! ładunku bez konieczności ręcznego kodowania masek bitowych i offsetów pakietu przez uczestników.

pub mod demux;
pub mod error;
pub mod header;
pub mod packet;

pub use demux::{classify_peeked_packet, PacketClass};
pub use error::RtpParseError;
pub use header::{RtpHeader, MIN_RTP_HEADER_LEN};
pub use packet::{build_test_rtp_packet, parse_rtp_header, RtpPacket};

