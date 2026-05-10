use std::sync::Arc;
use std::thread;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

use ag_core::bus::{BusError, MemoryBus};

const BUS_NAME: &str = "/ag-bus-bench";

fn setup() -> (Arc<MemoryBus>, Arc<MemoryBus>) {
    // Remove any leftover segment.
    let cname = std::ffi::CString::new(BUS_NAME).unwrap();
    unsafe { libc::shm_unlink(cname.as_ptr()) };

    let writer = Arc::new(MemoryBus::create(BUS_NAME).expect("create bench bus"));
    let reader = Arc::new(MemoryBus::open(BUS_NAME).expect("open bench bus"));
    (writer, reader)
}

/// Measures the single-threaded roundtrip latency: one write followed by one
/// read and response, all in the same thread (simulating the full state machine
/// without cross-thread communication).
fn bench_single_thread_roundtrip(c: &mut Criterion) {
    let (writer, _reader) = setup();

    let payload = vec![0u8; 256];
    let mut group = c.benchmark_group("bus/roundtrip");
    group.throughput(Throughput::Elements(1));

    group.bench_function("256B_payload", |b| {
        b.iter(|| {
            let idx = match writer.claim_write_slot() {
                Ok(i) => i,
                Err(BusError::BusFull(_)) => return,
                Err(e) => panic!("unexpected error: {e}"),
            };
            writer.write_request(idx, 1, 0, b"/bench", b"", &payload);
            // Simulate the Go side inline.
            let _req = writer.claim_read_slot();
            if let Some(read_idx) = writer.claim_read_slot() {
                writer.write_response(read_idx, 200, b"ok");
                let (status, _body) = writer.wait_response(idx);
                assert_eq!(status, 200);
            } else {
                // The slot was claimed for reading — simulate response directly.
                writer.write_response(idx, 200, b"ok");
                let (status, _body) = writer.wait_response(idx);
                assert_eq!(status, 200);
            }
        })
    });

    group.finish();
}

/// Measures write-side throughput: how many slots per second the Rust layer can
/// claim and populate with request data. This isolates the write path from
/// network and Go processing time.
fn bench_write_throughput(c: &mut Criterion) {
    let (writer, reader) = setup();
    let writer = Arc::clone(&writer);
    let reader = Arc::clone(&reader);

    // Background reader thread: drains READY slots and marks them DONE.
    let reader_handle = {
        let reader = Arc::clone(&reader);
        thread::spawn(move || {
            loop {
                if let Some(idx) = reader.claim_read_slot() {
                    reader.write_response(idx, 200, b"ok");
                } else {
                    std::hint::spin_loop();
                }
            }
        })
    };

    let payload = vec![0u8; 128];
    let mut group = c.benchmark_group("bus/write_throughput");
    group.throughput(Throughput::Elements(1));

    group.bench_function("128B_payload", |b| {
        b.iter(|| {
            match writer.claim_write_slot() {
                Ok(idx) => {
                    writer.write_request(idx, 1, 0, b"/bench", b"", &payload);
                    let _ = writer.wait_response(idx);
                }
                Err(BusError::BusFull(_)) => {}
                Err(e) => panic!("unexpected error: {e}"),
            }
        })
    });

    group.finish();

    // The reader thread is intentionally left running; criterion drops the
    // process after benchmarks complete.
    drop(reader_handle);
}

/// Measures slot capacity under concurrent writers.
fn bench_concurrent_writers(c: &mut Criterion) {
    const WRITER_COUNT: usize = 4;

    let (bus, _) = setup();
    let bus = Arc::new(bus);

    // Drain thread.
    {
        let bus = Arc::clone(&bus);
        thread::spawn(move || loop {
            if let Some(idx) = bus.claim_read_slot() {
                bus.write_response(idx, 200, b"ok");
            } else {
                std::hint::spin_loop();
            }
        });
    }

    let mut group = c.benchmark_group("bus/concurrent_writers");
    group.throughput(Throughput::Elements(WRITER_COUNT as u64));

    group.bench_with_input(
        BenchmarkId::new("writers", WRITER_COUNT),
        &WRITER_COUNT,
        |b, &count| {
            let bus = Arc::clone(&bus);
            b.iter(|| {
                let handles: Vec<_> = (0..count)
                    .map(|_| {
                        let bus = Arc::clone(&bus);
                        thread::spawn(move || {
                            if let Ok(idx) = bus.claim_write_slot() {
                                bus.write_request(idx, 1, 0, b"/bench", b"", b"{}");
                                let _ = bus.wait_response(idx);
                            }
                        })
                    })
                    .collect();
                for h in handles {
                    h.join().unwrap();
                }
            })
        },
    );

    group.finish();
}

criterion_group!(
    benches,
    bench_single_thread_roundtrip,
    bench_write_throughput,
    bench_concurrent_writers,
);
criterion_main!(benches);
