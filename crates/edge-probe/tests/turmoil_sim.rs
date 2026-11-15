//! Zadanie 11: deterministyczna symulacja usterek sieciowych w `turmoil`.

use std::time::Duration;
//
use turmoil::net::UdpSocket;

#[test]
fn turmoil_partition_and_replay_sim() -> turmoil::Result {
  let mut sim = turmoil::Builder::new()
    .min_message_latency(Duration::ZERO)
    .max_message_latency(Duration::ZERO)
    .build();

  sim.host("probe", || async {
    let _sock = UdpSocket::bind("0.0.0.0:5004").await?;
    // Zadanie 11: odbierz pakiety SRTP, zweryfikuj je przez SrtpVerifier (w tym Replay Protection)
    // oraz przekaż do SansIoRtpSession, sprawdzając roc, SendRtcpNack i SendRtcpReceiverReport.
    Ok(())
  });

  sim.client("sender", async {
    let sock = UdpSocket::bind("0.0.0.0:0").await?;
    turmoil::partition_oneway("sender", "probe");
    let _ = sock.send_to(b"...", "probe:5004").await;
    turmoil::repair("sender", "probe");
    Ok(())
  });

  sim.run()
}
