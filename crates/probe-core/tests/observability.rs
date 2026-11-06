use probe_core::observability::{
    init_tracing, record_rtp_gap_event, ActiveResourceGuard, ProbeDataplaneMetrics,
};

#[test]
fn test_init_tracing_and_event_recording() {
    let _guard = init_tracing("test_suite");

    // Zdarzenie ustrukturyzowane w drzewie spanów (zimna ścieżka)
    record_rtp_gap_event(0xCAFE_BABE, &[65535, 0], 1);

    // RAII guard dla metryk w locie
    {
        let _active = ActiveResourceGuard::new("test_in_flight_operations");
    }

    // Wywołania gorącej ścieżki metryk
    ProbeDataplaneMetrics::on_packet("rtp", 160, 20.5);
    ProbeDataplaneMetrics::on_rtp_gap_detected(2);
    ProbeDataplaneMetrics::on_srtp_auth_error();
}

