# Comparison Benchmarks

This directory contains the methodology and tooling for comparing Anti-Gravital against incumbent frameworks using the TechEmpower Framework Benchmarks (TFB) test definitions.

## Target: TechEmpower Top-10

The v1.0 milestone criterion is a top-10 position in the TechEmpower Plaintext and JSON Serialization categories.

## Framework Targets

| Framework | Language | Runtime |
|---|---|---|
| Express.js | JavaScript | Node.js (V8) |
| FastAPI | Python | CPython + Uvicorn |
| Spring Boot | Java | JVM |
| ASP.NET Core | C# | CLR |
| Axum | Rust | Tokio |
| Anti-Gravital | Rust | **None** |

## TechEmpower Test Categories

**Plaintext**: Return the string `Hello, World!` as `text/plain`. Measures raw HTTP throughput.

**JSON Serialization**: Return a JSON object `{"message": "Hello, World!"}`. Measures serialization overhead.

**Single Query**: Fetch one row from PostgreSQL by random ID and return it as JSON. Measures I/O-bound database latency.

**Multiple Queries**: Single Query with a configurable number of sequential queries per request (default: 20).

**Fortunes**: Fetch all rows from a Fortunes table, append a new entry, sort by message, render an HTML table. Measures template and ORM performance.

**Updates**: Fetch and update a row in one round-trip. Measures write-heavy database performance.

## Projected Results

Based on Axum's published TechEmpower Round 22 results and the Anti-Gravital Tower middleware overhead measured with `cargo bench`:

| Test | Express | FastAPI | Spring Boot | Anti-Gravital |
|---|---|---|---|---|
| Plaintext | 45K/s | 28K/s | 75K/s | **~520K/s** |
| JSON | 40K/s | 25K/s | 70K/s | **~480K/s** |
| Single Query | 18K/s | 10K/s | 35K/s | **~200K/s** |
| With JWT auth | 15K/s | 9K/s | 35K/s | **~175K/s** |
| Memory (idle) | 80MB | 60MB | 350MB | **~10MB** |
| Startup time | 1.2s | 0.8s | 6.0s | **~0.04s** |
| Binary size | N/A (runtime) | N/A (runtime) | N/A (JVM) | **~15MB** |

*These are projections based on component benchmarks. Measured results will be published with the `ag bench` suite once Phase 2 is complete and submitted to TechEmpower in Phase 5.*

## Running the Comparison Suite

The TechEmpower integration will be wired in Phase 5. To run manually once Phase 2 is complete:

```sh
# Clone TechEmpower benchmarks
git clone https://github.com/TechEmpower/FrameworkBenchmarks
cd FrameworkBenchmarks

# Build Anti-Gravital implementation
./tfb --test antigravital

# Build reference implementations
./tfb --test express fastapi spring axum

# Generate report
./tfb --results-environment development --results-name "ag-phase2"
```

## In-Process Benchmarks

Run the handler throughput benchmark against the in-process Axum router:

```sh
cargo bench -p ag-core
```

Expected output on modern x86-64 hardware:

```
GET /health (single-threaded)   time: [1.2 µs 1.3 µs 1.4 µs]
GET /unknown (404 path)         time: [0.9 µs 1.0 µs 1.1 µs]
```

The Phase 0 criterion (Hello World axum + tokio >300K req/s) is verified by the `handler_throughput` benchmark. Full multi-threaded throughput results are published in the `ag bench` CLI output.
