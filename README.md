# Anti-Gravital

A high-performance web framework built on Rust and Go that compiles to a single static binary. Anti-Gravital combines the memory safety and zero-cost abstractions of Rust with the concurrency model and developer ergonomics of Go, connected by a nanosecond-latency shared memory bus.

## What It Is

Anti-Gravital eliminates the traditional tradeoffs between performance and productivity in backend development. The framework is structured as a triad:

- **The Shield** (Rust): HTTP/TLS termination, request validation, and rate limiting. The Shield never allocates on the hot path and handles all network I/O using Tokio's async runtime.
- **The Brain** (Go): Business logic runtime. Go routines handle route dispatch, module execution, and response composition. The Brain is where application developers write their code.
- **The Memory Bus**: A POSIX shared memory ring buffer connecting the Shield and the Brain within the same machine. Slots transition through well-defined states (EMPTY -> WRITING -> READY -> READING -> DONE -> EMPTY) using lock-free atomic operations. Round-trip latency under load is sub-microsecond.

The result is a framework that targets TechEmpower Fortunes benchmark performance in the top tier while presenting a schema-first, type-safe development experience.

## Design Principles

**Zero-Overhead Abstraction**: Every layer of the framework is designed so that the abstraction costs nothing at runtime. The Rust HTTP layer does not heap-allocate per request. The shared memory bus does not copy data between layers.

**Single Binary**: A production deployment is a single executable with no external runtime, no JVM, no interpreter, no shared libraries beyond libc. The Go runtime is compiled in. The entire application, including TLS certificates and static assets, ships as one artifact.

**Schema First**: Application structure is defined in `.ag` schema files. The `ag` CLI generates type-safe handler stubs, validation logic, OpenAPI documentation, and database migration plans from the schema. The schema is the source of truth.

**Memory Safety**: The Rust layer provides memory safety guarantees for all network-facing code. The shared memory protocol is verified at compile time via size assertions and at runtime via magic number validation.

**Concurrency First**: The design assumes high concurrency from the start. The ring buffer supports multiple concurrent writers via compare-and-swap slot claiming. The Go Brain runs a configurable worker pool consuming the ring buffer.

## Architecture

```
                    Incoming Requests
                           |
                    +------v------+
                    |  The Shield |  (Rust / Tokio)
                    |  TLS + HTTP |
                    +------+------+
                           |  POSIX Shared Memory
                           |  Ring Buffer (64MB)
                    +------v------+
                    |  The Brain  |  (Go Runtime)
                    |  Goroutines |
                    +------+------+
                           |
               +-----------+-----------+
               |           |           |
           ag-auth      ag-data    ag-realtime   (Phase 2+)
```

The shared memory segment is 64MB organized as a header (256 bytes) followed by 8192-byte slots. The Rust side claims slots via atomic compare-and-swap, writes request data, and transitions the slot to READY. Go workers scan for READY slots, transition to READING, execute the handler, write the response, and transition to DONE. The Rust side spin-waits (with futex notification for CPU efficiency) for the DONE state, reads the response, and sends it to the client.

## Quick Start

### Prerequisites

- Rust 1.75 or newer (`rustup update stable`)
- Go 1.21 or newer
- Linux or macOS (Windows support planned for Phase 4)

### Installation

```sh
git clone https://github.com/angelnereira/anti-gravital
cd anti-gravital
cargo install --path ag-cli
```

### Create a New Project

```sh
ag new my-api --template rest
cd my-api
```

This creates:
```
my-api/
  schema.ag          # Your API schema
  src/
    handlers/        # Go handler implementations
  go.mod
```

### Define Your Schema

Edit `schema.ag`:

```
@version 1.0
@namespace api

model User {
    id       UUID
    name     String   @min(1) @max(100)
    email    String   @format(email)
    created  Timestamp
}

endpoint GetUser {
    method   GET
    path     /users/:id
    auth     required
    response User
}

endpoint CreateUser {
    method   POST
    path     /users
    body     CreateUserRequest
    response User
}
```

### Generate Code

```sh
ag generate
```

### Run in Development Mode

```sh
ag dev
```

### Build for Production

```sh
ag build --target x86_64-unknown-linux-musl
```

Produces a single static binary in `dist/`.

## Current Status: Phase 0 - Foundations

Phase 0 establishes the project skeleton, build toolchain, and the Memory Bus implementation.

| Phase | Status      | Description                              |
|-------|-------------|------------------------------------------|
| 0     | In Progress | Foundations: build system, Memory Bus   |
| 1     | Planned     | Shield: HTTP/TLS, routing, bus write    |
| 2     | Planned     | Brain: goroutine pool, modules          |
| 3     | Planned     | DSL: parser, code generator, CLI        |
| 4     | Planned     | Production: benchmarks, hardening       |

## Build Requirements

- Rust 1.75+ (`rustup toolchain install stable`)
- Go 1.21+
- For cross-compilation: `cross` (`cargo install cross`)
- For WASM plugins: `wasm-pack`

## Building

```sh
# Build the Rust components (ag-core library + ag CLI)
cargo build --release

# Build the Go runtime
cd ag-runtime
go build ./...

# Run tests
cargo test
cd ag-runtime && go test ./...

# Run benchmarks
cargo bench
cd ag-runtime && go test -bench=. -benchmem ./benchmarks/...
```

## License

Apache License 2.0. See [LICENSE](LICENSE).
