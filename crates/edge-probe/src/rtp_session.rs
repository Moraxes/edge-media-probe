//! Maszyna stanów *Sans-I/O* dla ciągłości sekwencji RTP oraz raportowania RTCP.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtocolOutput {
  ForwardRtp {
    ssrc: u32,
    seq: u16,
    roc: u32,
  },
  SendRtcpNack {
    media_ssrc: u32,
    missing_seqs: Vec<u16>,
  },
  SendRtcpReceiverReport {
    media_ssrc: u32,
    extended_highest_seq: u32,
    received_packets: u64,
    lost_packets: u64,
  },
}

/// Maszyna stanów *Sans-I/O* sesji RTP/RTCP.
pub struct SansIoRtpSession {
  pub media_ssrc: u32,
  pub highest_seq: Option<u16>,
  pub roc: u32,
  pub received_packets: u64,
  pub lost_packets: u64,
  pub next_rr_deadline: Instant,
  pub rr_interval: Duration,
  pub out_queue: VecDeque<ProtocolOutput>,
}

impl SansIoRtpSession {
  pub fn new(media_ssrc: u32, now: Instant, rr_interval: Duration) -> Self {
    Self {
      media_ssrc,
      highest_seq: None,
      roc: 0,
      received_packets: 0,
      lost_packets: 0,
      next_rr_deadline: now + rr_interval,
      rr_interval,
      out_queue: VecDeque::new(),
    }
  }

  pub fn handle_rtp(
    &mut self,
    version: u8,
    ssrc: u32,
    seq: u16,
    payload_len: usize,
  ) {
    if version != 2 || ssrc != self.media_ssrc || payload_len == 0 {
      return;
    }

    match self.highest_seq {
      None => self.highest_seq = Some(seq),
      Some(prev) => {
        let delta = seq.wrapping_sub(prev);
        if delta > 0 && delta < 0x8000 {
          if seq < prev {
            self.roc += 1;
          }
          if delta > 1 {
            let missing_seqs = (1..delta)
              .map(|offset| prev.wrapping_add(offset))
              .collect::<Vec<_>>();
            self.lost_packets += u64::from(delta - 1);
            self
              .out_queue
              .push_back(ProtocolOutput::SendRtcpNack {
                media_ssrc: ssrc,
                missing_seqs,
              });
          }
          self.highest_seq = Some(seq);
        }
      }
    }

    self.received_packets += 1;
    self
      .out_queue
      .push_back(ProtocolOutput::ForwardRtp {
        ssrc,
        seq,
        roc: self.roc,
      });
  }

  pub fn handle_timeout(&mut self, now: Instant) {
    if now >= self.next_rr_deadline {
      self.next_rr_deadline = now + self.rr_interval;
      let max_seq = self.highest_seq.unwrap_or(0);
      let extended_highest_seq = (self.roc << 16) | u32::from(max_seq);
      self
        .out_queue
        .push_back(ProtocolOutput::SendRtcpReceiverReport {
          media_ssrc: self.media_ssrc,
          extended_highest_seq,
          received_packets: self.received_packets,
          lost_packets: self.lost_packets,
        });
    }
  }

  pub fn poll_output(&mut self) -> Option<ProtocolOutput> {
    self.out_queue.pop_front()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn rtp_session_detects_wraparound_and_gap_nack() {
    let start = Instant::now();
    let mut session =
      SansIoRtpSession::new(0x1000, start, Duration::from_secs(5));

    session.handle_rtp(2, 0x1000, 65535, 100);
    assert_eq!(
      session.poll_output(),
      Some(ProtocolOutput::ForwardRtp {
        ssrc: 0x1000,
        seq: 65535,
        roc: 0,
      })
    );

    // Zawinięcie 65535 -> 1 oznacza utratę seq = 0 oraz wzrost ROC do 1.
    session.handle_rtp(2, 0x1000, 1, 100);
    assert_eq!(
      session.poll_output(),
      Some(ProtocolOutput::SendRtcpNack {
        media_ssrc: 0x1000,
        missing_seqs: vec![0],
      })
    );
    assert_eq!(
      session.poll_output(),
      Some(ProtocolOutput::ForwardRtp {
        ssrc: 0x1000,
        seq: 1,
        roc: 1,
      })
    );

    session.handle_timeout(start + Duration::from_secs(5));
    assert_eq!(
      session.poll_output(),
      Some(ProtocolOutput::SendRtcpReceiverReport {
        media_ssrc: 0x1000,
        extended_highest_seq: (1 << 16) | 1,
        received_packets: 2,
        lost_packets: 1,
      })
    );
  }
}
