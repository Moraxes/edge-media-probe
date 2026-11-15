use bytes::Bytes;
use rtp_parser::{
  PacketClass, RtpParseError, build_test_rtp_packet, classify_peeked_packet,
  parse_rtp_header,
};

#[test]
fn test_classify_rtp_and_rtcp() {
  // RTP: wersja 2, PT 96
  let rtp_bytes = [0x80, 96, 0x00, 0x01];
  assert_eq!(classify_peeked_packet(&rtp_bytes), Some(PacketClass::Rtp));

  // RTCP: wersja 2, PT 200 (Sender Report)
  let rtcp_sr = [0x80, 200, 0x00, 0x06];
  assert_eq!(classify_peeked_packet(&rtcp_sr), Some(PacketClass::Rtcp));

  // RTCP: wersja 2, PT 205 (RTPFB / Generic NACK)
  let rtcp_nack = [0x80, 205, 0x00, 0x03];
  assert_eq!(classify_peeked_packet(&rtcp_nack), Some(PacketClass::Rtcp));

  // Nieprawidłowa wersja RTP
  let invalid_version = [0x00, 96, 0x00, 0x01];
  assert_eq!(classify_peeked_packet(&invalid_version), None);

  // Za krótki bufor
  assert_eq!(classify_peeked_packet(&[0x80]), None);
}

#[test]
fn test_parse_rtp_header_basic() {
  let raw = build_test_rtp_packet(42, 160_000, 0xCAFE_BABE, &[0xAA; 160]);
  let (hdr, payload) = parse_rtp_header(raw).expect("Poprawne parsowanie RTP");

  assert_eq!(hdr.version, 2);
  assert_eq!(hdr.sequence_number, 42);
  assert_eq!(hdr.timestamp, 160_000);
  assert_eq!(hdr.ssrc, 0xCAFE_BABE);
  assert_eq!(hdr.payload_type, 96);
  assert!(!hdr.marker);
  assert_eq!(payload.len(), 160);
  assert_eq!(payload[0], 0xAA);
}

#[test]
fn test_parse_errors() {
  // Zbyt krótki pakiet
  let err =
    parse_rtp_header(Bytes::from_static(&[0x80, 96, 0x01])).unwrap_err();
  assert!(matches!(err, RtpParseError::PacketTooShort { .. }));

  // Błędna wersja
  let bad_ver = [0x40, 96, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0];
  let err = parse_rtp_header(Bytes::copy_from_slice(&bad_ver)).unwrap_err();
  assert!(matches!(err, RtpParseError::InvalidVersion(1)));
}

#[test]
fn test_interop_with_webrtc_rtp_crate() {
  use rtp::header::Header;
  use rtp::packet::Packet;
  use webrtc_util::marshal::Marshal;

  let webrtc_pkt = Packet {
    header: Header {
      version: 2,
      padding: false,
      extension: false,
      marker: true,
      payload_type: 111,
      sequence_number: 999,
      timestamp: 48000,
      ssrc: 0xDEAD_BEEF,
      csrc: vec![0x1111_2222],
      ..Default::default()
    },
    payload: Bytes::from_static(b"OPUS_AUDIO_SAMPLE"),
  };

  let raw = webrtc_pkt
    .marshal()
    .expect("Serializacja webrtc-rs");
  let (hdr, payload) = parse_rtp_header(raw).expect("Parsowanie w rtp-parser");

  assert_eq!(hdr.version, 2);
  assert!(hdr.marker);
  assert_eq!(hdr.payload_type, 111);
  assert_eq!(hdr.sequence_number, 999);
  assert_eq!(hdr.timestamp, 48000);
  assert_eq!(hdr.ssrc, 0xDEAD_BEEF);
  assert_eq!(hdr.csrc, vec![0x1111_2222]);
  assert_eq!(&payload[..], b"OPUS_AUDIO_SAMPLE");
}
