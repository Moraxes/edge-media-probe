use std::hint::black_box;
use std::thread;

use bytes::{BufMut, Bytes, BytesMut};
use criterion::{criterion_group, criterion_main, Criterion};

#[cfg(target_arch = "aarch64")]
const CACHE_LINE: usize = 128;
#[cfg(not(target_arch = "aarch64"))]
const CACHE_LINE: usize = 64;

const THREADS: usize = 4;
const CLONE_ITERS: usize = 50_000;
const RW_ITERS: usize = 2_000_000;

fn clone_drop_loop(slice: &Bytes) {
  for _ in 0..CLONE_ITERS {
    let cloned = black_box(slice).clone();
    black_box(cloned);
  }
}

/// Porównuje rywalizację 4 wątków o wspólny licznik referencji `Shared`
/// względem niezależnych buforów `Bytes` z własnymi licznikami.
fn bench_shared_refcount(c: &mut Criterion) {
  let mut group = c.benchmark_group("bytes_shared_refcount");

  // Wszystkie wycinki pochodzą z jednej puli, więc dzielą ten sam AtomicUsize.
  let mut pool = BytesMut::with_capacity(THREADS * 64);
  pool.put_bytes(0xAA, THREADS * 64);
  let shared_slices: Vec<Bytes> = (0..THREADS)
    .map(|_| pool.split_to(64).freeze())
    .collect();

  group.bench_function("shared_pool_4_threads", |b| {
    b.iter(|| {
      thread::scope(|s| {
        for slice in &shared_slices {
          s.spawn(move || clone_drop_loop(slice));
        }
      });
    });
  });

  // Każdy wątek dostaje osobny bufor z własnym licznikiem referencji.
  let separate_slices: Vec<Bytes> = (0..THREADS)
    .map(|_| {
      let mut buf = BytesMut::with_capacity(64);
      buf.put_bytes(0xAA, 64);
      buf.freeze()
    })
    .collect();

  group.bench_function("separate_buffers_4_threads", |b| {
    b.iter(|| {
      thread::scope(|s| {
        for slice in &separate_slices {
          s.spawn(move || clone_drop_loop(slice));
        }
      });
    });
  });

  group.finish();
}

/// Przesuwa początek `pool` na wielokrotność `CACHE_LINE`.
fn align_start(pool: &mut BytesMut) {
  let misalign = (pool.as_ptr() as usize) % CACHE_LINE;
  if misalign != 0 {
    let lead = CACHE_LINE - misalign;
    pool.put_bytes(0, lead);
    let _ = pool.split_to(lead);
  }
}

/// Jeden wątek czyta pierwszy bajt `reader`, a drugi nadpisuje pierwszy bajt `writer`.
fn hammer_adjacent_bytes(reader: &Bytes, writer: &mut BytesMut) {
  thread::scope(|s| {
    s.spawn(|| {
      let mut acc = 0u8;
      for _ in 0..RW_ITERS {
        acc = acc.wrapping_add(black_box(reader[0]));
      }
      black_box(acc);
    });

    for i in 0..RW_ITERS {
      writer[0] = black_box(i as u8);
    }
  });
}

/// Porównuje odczyt `Bytes` i zapis w `BytesMut` po `split_to`:
/// układ `[reader 32 B | writer 32 B]` (ta sama linia cache) vs
/// układ `[reader 32 B | odstęp | writer 32 B]` (osobne linie cache).
fn bench_split_to_boundary(c: &mut Criterion) {
  let mut group = c.benchmark_group("bytes_split_to_boundary");

  // 1. Bez odstępu: reader [0..32) i writer [32..64) leżą w jednej linii cache.
  let mut unaligned = BytesMut::with_capacity(CACHE_LINE * 2);
  align_start(&mut unaligned);
  unaligned.put_bytes(1, 64);
  let unaligned_reader = unaligned.split_to(32).freeze();

  group.bench_function("unaligned_same_cache_line", |b| {
    b.iter(|| hammer_adjacent_bytes(&unaligned_reader, &mut unaligned));
  });

  // 2. Z odstępem: odrzucamy (CACHE_LINE - 32) bajtów, rozdzielając linie cache.
  let mut aligned = BytesMut::with_capacity(CACHE_LINE * 3);
  align_start(&mut aligned);
  aligned.put_bytes(1, CACHE_LINE + 32);
  let aligned_reader = aligned.split_to(32).freeze();
  let _padding = aligned.split_to(CACHE_LINE - 32);

  group.bench_function("aligned_separate_cache_lines", |b| {
    b.iter(|| hammer_adjacent_bytes(&aligned_reader, &mut aligned));
  });

  group.finish();
}

criterion_group!(benches, bench_shared_refcount, bench_split_to_boundary);
criterion_main!(benches);
