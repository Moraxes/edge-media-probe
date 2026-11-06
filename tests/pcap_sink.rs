use edge_media_probe::pcap_sink::{record_captured_frames, record_captured_stream, PortableUringStyleSink};
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

#[tokio::test]
async fn test_record_captured_stream_with_combinators() -> io::Result<()> {
    let tmp_path = std::env::temp_dir().join("edge_probe_stream_dump.bin");
    let mut sink = PortableUringStyleSink::create(&tmp_path).await?;

    let frames = vec![
        bytes::Bytes::from_static(b"STREAM_FRAME_1"),
        bytes::Bytes::from_static(b"STREAM_FRAME_2"),
        bytes::Bytes::from_static(b"STREAM_FRAME_3"),
    ];
    let frame_stream = tokio_stream::iter(frames);

    let total_bytes = record_captured_stream(&mut sink, frame_stream).await?;
    assert!(total_bytes > 0);

    let on_disk = tokio::fs::read(&tmp_path).await?;
    assert_eq!(on_disk.len() as u64, total_bytes);
    let _ = tokio::fs::remove_file(&tmp_path).await;
    Ok(())
}

