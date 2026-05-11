# Anti-Gravital

A high-performance web framework written entirely in Rust. Anti-Gravital builds on Axum, Tower, and Tokio to deliver a schema-first, type-safe development experience without sacrificing throughput.

## What It Is

Anti-Gravital eliminates the traditional tradeoffs between performance and productivity in backend development. The framework is structured as two cooperating layers:

- **The Shield** (Tower middleware): HTTP request validation, JWT verification, rate limiting, and CORS. Implemented as a composable Tower `Layer` that wraps any Axum router. Schema contracts compiled from `.ag` files enforce the API surface at the boundary — invalid requests never reach handler code.
- **The Core** (Axum handlers): Type-safe handler functions, structured errors that serialize to consistent JSON, and request context extraction. Application developers write handler functions here; the framework handles routing, serialization, and observability.

The result is a single-binary framework that targets TechEmpower benchmark performance in the top tier while presenting a schema-first, type-safe development experience.

## Design Principles

**Zero-Overhead Abstraction**: The Rust HTTP layer does not heap-allocate on the hot path. Tower middleware composes at compile time; the cost of the full middleware stack is a single virtual dispatch.

**Single Binary**: A production deployment is one static executable. No external runtime, no JVM, no interpreter. The entire application, including TLS certificates and embedded assets, ships as one artifact.

**Schema First**: Application structure is defined in `.ag` schema files. The `ag` CLI generates type-safe Rust structs, TypeScript interfaces, and OpenAPI 3.1 documentation from the schema. The schema is the source of truth; type drift between frontend and backend becomes a compile error.

**Async Native**: The framework is built on Tokio's async runtime. Every handler is a `Future`. There are no blocking thread pools, no CGO bridges, and no shared memory segments — just stackless async tasks scheduled by Tokio.

## Architecture

```
                    Incoming Requests
                           |
              +------------v-------------+
              |       Tower Stack        |
              |                          |
              |  request-id middleware   |
              |  trace middleware        |
              |  timeout middleware      |
              |  compression middleware  |
              |  cors middleware         |
              |                          |
              |  ShieldLayer             |  Schema validation
              |  (Phase 1+)              |  JWT verification
              |                          |  Rate limiting
              +------------+-------------+
                           |
              +------------v-------------+
              |       Axum Router        |
              |                          |
              |  GET /health             |  Core handlers
              |  GET /metrics            |
              |  ...application routes   |
              +------------+-------------+
                           |
                    HTTP Response
```

The Shield and Core share a single OS thread pool managed by Tokio. There is no inter-process communication, no shared memory segment, and no serialization cost between layers.

## Quick Start

### Prerequisites

- Rust 1.75 or newer (`rustup update stable`)

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

### Define Your Schema

Edit `schema.ag`:

```
model User {
  id:    UUID      @primary
  email: Email     @unique
  name:  String
}

endpoint GET /users/{id} -> User
  auth: jwt
  validate: strict
```

### Generate Code

```sh
ag generate --write
```

This writes:
- `src/generated/models.rs` — Serde-annotated Rust structs
- `src/generated/models.ts` — TypeScript interfaces
- `openapi.json` — OpenAPI 3.1 specification

### Run in Development Mode

```sh
ag dev
```

### Inspect a Running Server

```sh
# Health check
curl http://localhost:3000/health

# Metrics
curl http://localhost:3000/metrics

# Benchmark
ag bench --endpoint /health --concurrency 100 --requests 10000
```

### Build for Production

```sh
ag build --release
```

Produces a single static binary in `target/release/`.

## Current Status: Phase 1

Phase 0 (pure-Rust foundation with Axum + Tokio) is complete. Phase 1 is in progress.

| Phase | Status      | Description                                        |
|-------|-------------|----------------------------------------------------|
| 0     | Complete    | Axum + Tokio skeleton, Shield Tower layer, CLI     |
| 1     | In Progress | JWT auth, schema validation, ShieldLayer populated |
| 2     | Planned     | sqlx integration, async-nats, moka cache           |
| 3     | Planned     | DSL code generator, deploy tooling                 |
| 4     | Planned     | WASM plugins, tokio-console integration            |

## Building

```sh
# Build all crates
cargo build --workspace

# Run tests
cargo test --workspace

# Run benchmarks (requires a Criterion baseline)
cargo bench -p ag-core

# Build release binary
cargo build --release -p ag
```

## Workspace Layout

```
anti-gravital/
  ag-core/        # Shield middleware + Core handlers + Axum server
  ag-dsl/         # .ag schema lexer, parser, semantic checker, and code generators
  ag-cli/         # `ag` command-line tool
  ag-wasm/        # WebAssembly plugin host (Phase 4)
  examples/       # Runnable example applications
  docs/           # Architecture and API documentation
```

## License

Apache License 2.0. See [LICENSE](LICENSE).
