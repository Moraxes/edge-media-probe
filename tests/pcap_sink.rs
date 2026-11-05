use edge_media_probe::pcap_sink::{record_captured_frames, PortableUringStyleSink};
use std::io;

#[tokio::test]
async fn test_completion_based_file_sink_recycles_buffer() -> io::Result<()> {
    let tmp_path = std::env::temp_dir().join("edge_probe_ring_dump.bin");
    let mut sink = PortableUringStyleSink::create(&tmp_path).await?;

    let sample_frames: &[&[u8]] = &[
        b"RTP_FRAME_SEQ_1",
        b"RTP_FRAME_SEQ_2",
        b"RTCP_NACK_PACKET",
    ];

    let total_bytes = record_captured_frames(&mut sink, sample_frames).await?;
    assert!(total_bytes > 0);

    let on_disk = tokio::fs::read(&tmp_path).await?;
    assert_eq!(on_disk.len() as u64, total_bytes);
    let _ = tokio::fs::remove_file(&tmp_path).await;
    Ok(())
}

