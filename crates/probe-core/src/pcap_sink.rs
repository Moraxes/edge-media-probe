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
        // Zadanie 1.4: Zaimplementuj asynchroniczny zapis plikowy ze zwrotem własności bufora:
        // Wymagania:
        // 1. Wykonaj przesunięcie offsetu w pliku: `self.file.seek(io::SeekFrom::Start(offset)).await`.
        // 2. Zapisz zawartość: `self.file.write_all(&buf).await`.
        // 3. Zwróć parę: (wynik_zapisu, zwrócony_bufor). Bufor `buf` musi wrócić do wywołującego niezależnie od wyniku.
        todo!("Zadanie 1.4: Zaimplementuj write_all_at w modelu completion-based")
    }

    async fn sync_data(&mut self) -> io::Result<()> {
        // Zadanie 1.4: Zsynchronizuj dane pliku z dyskiem
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
    // Zadanie 1.4: Zaimplementuj pętlę asynchronicznego zapisu ramek ze strumienia:
    // Wymagania:
    // 1. Zaalokuj jeden bufor roboczy wielokrotnego użytku: `reusable_buf = Vec::with_capacity(2048)`.
    // 2. W pętli `while let Some(frame) = stream.next().await`:
    //    - Wyczyść bufor `reusable_buf.clear()`,
    //    - Zapisz 4-bajtową długość ramki (little-endian) oraz bajty ramki,
    //    - Wywołaj `sink.write_all_at(offset, reusable_buf).await`,
    //    - Zaktualizuj offset i przejmij z powrotem zwrócony bufor (`reusable_buf = returned_buf`),
    //    - Unikaj jakichkolwiek nowych alokacji na stercie wewnątrz pętli.
    // 3. Po zakończeniu strumienia wywołaj `sink.sync_data().await?` i zwróć całkowity zapisany offset.
    todo!("Zadanie 1.4: Zaimplementuj record_captured_stream z recyklingiem bufora")
}

/// Pętla zapisu wycinka ramek oparta na strumieniowym zapisie z recyklingiem bufora
pub async fn record_captured_frames<S: AsyncFileSink>(
    sink: &mut S,
    frames: &[&[u8]],
) -> io::Result<u64> {
    let frame_stream = tokio_stream::iter(frames.iter().map(|f| bytes::Bytes::copy_from_slice(f)));
    record_captured_stream(sink, frame_stream).await
}
