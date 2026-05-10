# Modules

Anti-Gravital provides a set of batteries-included modules that cover common
application concerns. Each module is implemented in Go and communicates with
the Shield layer through the Memory Bus.

Modules are implemented in Phases 2 through 4. This directory will be
populated as each phase completes.

## Planned Modules

### ag-auth

Authentication and authorization.

- Passkeys / WebAuthn (FIDO2) out of the box
- JWT signing and verification with Ed25519
- Role-based access control (RBAC) with policies defined in the `.ag` schema
- Rate limiting per identity (complements the IP-level rate limiting in the Shield)
- Refresh token rotation with configurable expiry
- OAuth2 client: Google, GitHub, Gravital ID

### ag-data

Type-safe database access.

- Code-generated queries from the `.ag` schema using sqlc
- PostgreSQL (pgx connection pool), SQLite, and MySQL
- Database migrations managed by Goose, generated from schema changes
- Multi-tenant schema-per-tenant isolation
- Read replica routing for SELECT queries

### ag-realtime

Real-time event bus.

- Embedded NATS server (no external dependency for single-node deployments)
- WebSocket server with binary protocol for efficiency
- Server-Sent Events (SSE) fallback for environments that block WebSocket
- Event types generated from the `.ag` schema
- JetStream persistence for durable event delivery

### ag-cache

In-process and distributed caching.

- LRU and LFU caches backed by Rust (zero network overhead)
- Redis adapter for distributed deployments
- Automatic cache invalidation via AG-Realtime events
- SQL query result caching with table-level invalidation

### ag-storage

Object storage with a consistent interface.

- Adapters for S3, MinIO (Gravital Cloud default), and local filesystem
- Signed URL generation for client-side uploads and downloads
- In-process image processing (resize, compress, format conversion) via Rust
- CDN-ready response headers

### ag-observability

Distributed tracing and metrics.

- OpenTelemetry traces propagated automatically across the Rust and Go layers
- Prometheus metrics endpoint at `/metrics`
- Per-endpoint latency histograms at p50, p95, p99
- Pre-built Grafana dashboard definition included
- CPU and memory profiling without runtime overhead (pprof in dev, disabled in release)

## Writing a Custom Module

A module is any Go package that accepts a `brain.Context` and returns an error.
There is no registration mechanism beyond adding your package as a dependency
and calling it from your handler. The framework does not impose a plugin
interface in the Go layer — composition is done with ordinary Go imports.

For WASM plugins (running at the Shield layer in Rust), see `ag-wasm/`.
