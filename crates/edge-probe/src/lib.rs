#![allow(unused_imports, unused_variables, dead_code)]

pub use rtp_parser;

pub mod control_plane;
pub mod kms_client;
pub mod orchestrator;

/// Parsuje numer SSRC z formatu szesnastkowego (`0x...`) lub dziesiętnego.
pub fn parse_ssrc(raw: &str) -> Option<u32> {
    if let Some(hex) = raw.strip_prefix("0x").or_else(|| raw.strip_prefix("0X")) {
        u32::from_str_radix(hex, 16).ok()
    } else {
        raw.parse::<u32>().ok()
    }
}
