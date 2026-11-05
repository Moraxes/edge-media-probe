//! Moduł 1 (Zadanie 1.4): Asynchroniczny zapis plikowy w modelu Completion-Based (`io_uring`).
//!
//! Ćwiczenie obejmuje:
//! - Projekt traitu `AsyncFileSink` (AFIT z Rust 2024),
//! - Przekazywanie i zwrot własności bufora `Vec<u8>` (ochrona przed Use-After-Free i 0 alokacji),
//! - Implementację dla Linuksa (`tokio-uring`) oraz wersję przenośną (`PortableUringStyleSink`).

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
    async fn write_all_at(&mut self, offset: u64, buf: Vec<u8>) -> (io::Result<usize>, Vec<u8>) {
        use tokio::io::{AsyncSeekExt, AsyncWriteExt};
        let res = async {
            self.file.seek(io::SeekFrom::Start(offset)).await?;
            self.file.write_all(&buf).await?;
            Ok(buf.len())
        }
        .await;
        (res, buf)
    }

    async fn sync_data(&mut self) -> io::Result<()> {
        self.file.sync_data().await
    }
}

/// Pętla zapisu ramek z recyklingiem pojedynczego bufora
pub async fn record_captured_frames<S: AsyncFileSink>(
    sink: &mut S,
    frames: &[&[u8]],
) -> io::Result<u64> {
    let mut reusable_buf = Vec::with_capacity(2048);
    let initial_ptr = reusable_buf.as_ptr();
    let mut offset = 0u64;

    for frame in frames {
        reusable_buf.clear();
        reusable_buf.extend_from_slice(&(frame.len() as u32).to_le_bytes());
        reusable_buf.extend_from_slice(frame);

        let (written_res, returned_buf) = sink.write_all_at(offset, reusable_buf).await;
        let written = written_res?;
        offset += written as u64;
        reusable_buf = returned_buf;

        debug_assert_eq!(
            reusable_buf.as_ptr(),
            initial_ptr,
            "Bufor powinien być wielokrotnie używany bez nowej alokacji na stercie!"
        );
    }

    sink.sync_data().await?;
    Ok(offset)
}


