use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum RtpParseError {
  #[error(
    "Datagram jest zbyt krótki dla minimalnego nagłówka RTP: wymagane {needed} B, otrzymano {got} B"
  )]
  PacketTooShort { needed: usize, got: usize },

  #[error("Nieobsługiwana wersja RTP: {0} (oczekiwano 2 zgodnie z RFC 3550)")]
  InvalidVersion(u8),

  #[error("Ucięta lista CSRC: wymagane {needed} B, pozostało {got} B")]
  TruncatedCsrc { needed: usize, got: usize },

  #[error("Ucięty nagłówek rozszerzenia RTP (Extension Header)")]
  TruncatedExtension,

  #[error(
    "Nieprawidłowy padding RTP: rozmiar dopełnienia {padding_len} B przekracza długość ładunku {payload_len} B"
  )]
  InvalidPadding {
    padding_len: usize,
    payload_len: usize,
  },
}
