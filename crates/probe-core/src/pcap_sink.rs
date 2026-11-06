//! Moduł 1 (Zadanie 1.4): Asynchroniczny zapis plikowy w modelu Completion-Based (`io_uring`).
//!
//! Ćwiczenie obejmuje:
//! - Projekt traitu `AsyncFileSink` (AFIT z Rust 2024),
//! - Przekazywanie i zwrot własności bufora `Vec<u8>` (ochrona przed Use-After-Free i 0 alokacji),
//! - Implementację dla Linuksa (`tokio-uring`) oraz wersję przenośną (`PortableUringStyleSink`).

use futures_util::{Stream, StreamExt};
use std::io;
use std::path::Path;

/// Trait asynchronicznego I/O w modelu completion-based
pub trait AsyncFileSink {
    /// Przejmuje własność bufora na czas trwania zapisu i zwraca go po zakończeniu:
    async fn write_all_at(&mut self, offset: u64, buf: Vec<u8>) -> (io::Result<usize>, Vec<u8>);

    async fn sync_data(&mut self) -> io::Result<()>;
}

/// Implementacja natywna dla Linuksa oparta na pierścieniach `io_uring` (`tokio-uring`).
#[cfg(target_os = "linux")]
pub struct UringFileSink {
    file: tokio_uring::fs::File,
}

#[cfg(target_os = "linux")]
impl UringFileSink {
    pub async fn create(path: impl AsRef<Path>) -> io::Result<Self> {
        let file = tokio_uring::fs::File::create(path).await?;
        Ok(Self { file })
    }
}

#[cfg(target_os = "linux")]
impl AsyncFileSink for UringFileSink {
    async fn write_all_at(&mut self, offset: u64, buf: Vec<u8>) -> (io::Result<usize>, Vec<u8>) {
        let (res, buf) = self.file.write_at(buf, offset).await;
        (res, buf)
    }

    async fn sync_data(&mut self) -> io::Result<()> {
        self.file.sync_data().await
    }
}

/// Przenośna implementacja o identycznym kontrakcie własności bufora (macOS, Windows, Linux fallback).
pub struct PortableUringStyleSink {
    file: tokio::fs::File,
}

impl PortableUringStyleSink {
    pub async fn create(path: impl AsRef<Path>) -> io::Result<Self> {
        let file = tokio::fs::File::create(path).await?;
        Ok(Self { file })
    }
}

impl AsyncFileSink for PortableUringStyleSink {
    async fn write_all_at(&mut self, _offset: u64, _buf: Vec<u8>) -> (io::Result<usize>, Vec<u8>) {
        // Zadanie 1.4: Asynchroniczny zapis plikowy ze zwrotem własności bufora
        todo!()
    }

    async fn sync_data(&mut self) -> io::Result<()> {
        self.file.sync_data().await
    }
}

/// Pętla zapisu ramek ze strumienia z recyklingiem pojedynczego bufora
pub async fn record_captured_stream<S, St>(
    _sink: &mut S,
    _stream: St,
) -> io::Result<u64>
where
    S: AsyncFileSink,
    St: Stream<Item = bytes::Bytes> + Unpin,
{
    // Zadanie 1.4: Pętla zapisu ramek ze strumienia z recyklingiem bufora
    todo!()
}

/// Pętla zapisu wycinka ramek oparta na strumieniowym zapisie z recyklingiem bufora
pub async fn record_captured_frames<S: AsyncFileSink>(
    sink: &mut S,
    frames: &[&[u8]],
) -> io::Result<u64> {
    let frame_stream = tokio_stream::iter(frames.iter().map(|f| bytes::Bytes::copy_from_slice(f)));
    record_captured_stream(sink, frame_stream).await
}
