# Comparison Benchmarks

This directory contains the methodology and tooling for comparing Anti-Gravital
against incumbent frameworks using the TechEmpower Framework Benchmarks (TFB)
test definitions.

## Framework Targets

| Framework     | Language   | Runtime            |
|---------------|------------|--------------------|
| Express.js    | JavaScript | Node.js (V8)       |
| FastAPI       | Python     | CPython + Uvicorn  |
| Spring Boot   | Java       | JVM (GraalVM AOT)  |
| Gin           | Go         | Go runtime         |
| Axum          | Rust       | Tokio              |
| Anti-Gravital | Rust + Go  | None               |

## TechEmpower Test Categories

The following categories are tested:

**JSON Serialization**: Return a JSON object `{"message": "Hello, World!"}`.
Measures pure serialization overhead.

**Plaintext**: Return the string `Hello, World!` as `text/plain`.
Measures raw HTTP throughput.

**Single Query**: Fetch one row from a PostgreSQL table by random ID and return
it as JSON. Measures I/O-bound database latency.

**Multiple Queries**: Same as Single Query but with a configurable number of
sequential queries per request (default: 20).

**Fortunes**: Fetch all rows from a Fortunes table, append a new entry, sort
by message, and render an HTML table. Measures ORM and template performance.

**Updates**: Fetch and update a row in one round-trip. Measures write-heavy
database performance.

## Running the Comparison Suite

The comparison suite is not automated in Phase 0. It will be wired up in Phase 4
once the framework has a complete HTTP server implementation.

To run manually once Phase 1 is complete:

```sh
# Clone TechEmpower benchmarks
git clone https://github.com/TechEmpower/FrameworkBenchmarks
cd FrameworkBenchmarks

# Build Anti-Gravital implementation
./tfb --test antigravital

# Build reference implementations
./tfb --test express fastapi spring gin axum

# Generate comparison report
./tfb --results-environment development --results-name "ag-phase1"
```

## Projected Results (Phase 0 Estimates)

Based on component benchmarks from Axum (TechEmpower Round 22) and Gin:

| Test          | Express | FastAPI | Spring | Gin    | Axum   | Anti-Gravital |
|---------------|---------|---------|--------|--------|--------|---------------|
| Plaintext     | 45K/s   | 28K/s   | 75K/s  | 320K/s | 500K/s | 550K/s        |
| JSON          | 40K/s   | 25K/s   | 70K/s  | 280K/s | 480K/s | 520K/s        |
| Single Query  | 18K/s   | 10K/s   | 35K/s  | 120K/s | 180K/s | 200K/s        |
| Memory (idle) | 80 MB   | 60 MB   | 350 MB | 8 MB   | 10 MB  | 12 MB         |
| Startup time  | 1.2s    | 0.8s    | 6.0s   | 0.03s  | 0.04s  | 0.05s         |

*These are projections, not measured results. Actual benchmarks will be
published in Phase 4 with the full `ag bench` CLI integration.*

## Memory Bus Latency Baseline

The foundational claim of Anti-Gravital is sub-microsecond Rust-to-Go
communication. Run the Rust-side benchmark to verify the baseline:

```sh
cargo bench --package ag-core
```

Expected output on modern x86-64 hardware (Intel Core i9 or AMD Ryzen 9):

```
bus/roundtrip/256B_payload  time: [480 ns 495 ns 510 ns]
bus/write_throughput/128B   time: [350 ns 360 ns 370 ns]
```

The roundtrip target for Phase 0 is under 1 microsecond. The combined
Shield+Brain target for Phase 1 is under 5 microseconds of framework overhead
per request (excluding database and application code).
