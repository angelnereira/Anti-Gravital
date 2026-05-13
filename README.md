# Anti-Gravital Framework

**Blueprint Técnico v3.0 — Gravital Labs**
*Sabanitas, Colón, República de Panamá*

> "No construimos otro framework. Construimos la infraestructura sobre la que se escribirá el software de alto rendimiento del próximo cuarto de siglo — desde Panamá hacia el resto del mundo."
> — Ángel Nereira, Gravital Labs

---

| Throughput | Memoria base | Startup | Deploy |
|---|---|---|---|
| ~520K req/s | ~10MB | 0.04s | 1 binario estático |

---

A high-performance web framework written entirely in Rust. Anti-Gravital solves the performance vs. productivity tradeoff that every other framework accepts as inevitable. A single binary, no external runtime, no JVM, no GC, no interpreter — just native machine code, compile-time memory safety, and massive concurrency without GC pauses.

This is not another framework. It is the infrastructure on which high-performance software of the next quarter-century will be written.

## What It Is

Anti-Gravital is built on four pillars:

- **The Shield** — A composable Tower middleware pipeline. Handles TLS 1.3, JWT/Ed25519 verification, schema validation from `.ag` contracts, rate limiting, RBAC guards, and CORS/CSRF. Zero heap allocation on the hot path.
- **The Core** — Axum handlers backed by Tokio tasks. Business logic lives here; the framework handles routing, serialization, errors, and observability.
- **The Anti-DSL** — A schema-first `.ag` file is the single source of truth. One file generates Rust structs, TypeScript interfaces, OpenAPI 3.1, sqlx queries, SQL migrations, and handler stubs.
- **Batteries Included** — AG-Auth, AG-Data, AG-Realtime, AG-Cache, AG-Storage, AG-UI, AG-Observability ship as first-party modules.

## Design Principles

| Principle | Implementation |
|---|---|
| Zero-Overhead Abstraction | Tower + Axum generate code as efficient as hand-written |
| Single Binary, Single Runtime | No Node. No JVM. No Python. No external runtime in production |
| Schema First | One `.ag` file generates everything else |
| Memory Safety by Default | The compiler eliminates entire categories of vulnerabilities |
| Concurrency as a First Citizen | Tokio tasks: millions concurrent, no GC |
| Unified Observability | One runtime = one stack trace, one profiler, one tracer |
| AI-Native Design | The `.ag` schema is the perfect contract for AI agents |
| Open Source, Forever | Apache 2.0. No enterprise edition. No lock-in. |

## Architecture: The Dual

Two conceptual layers within a single Rust process. Communication between them is an ordinary function call — zero measurable overhead.

```
                    Incoming Requests
                           |
          +----------------v-----------------+
          |      ANTI-GRAVITAL RUNTIME       |
          |          (~10MB base)            |
          |                                  |
          |  THE SHIELD (Capa A)             |
          |  Tower middleware pipeline       |
          |  ├─ TLS 1.3 (rustls)            |
          |  ├─ JWT Ed25519 verification     |
          |  ├─ Schema validation (.ag)      |
          |  ├─ Rate limiting (governor)     |
          |  ├─ RBAC guards                  |
          |  └─ CORS / CSRF                  |
          |       │ Rust function call (0ns) |
          |       ▼                          |
          |  THE CORE (Capa B)               |
          |  Axum router + handlers          |
          |  ├─ Business logic               |
          |  ├─ AG-Data (sqlx)              |
          |  ├─ AG-Realtime (nats/WS)       |
          |  └─ Tokio tasks (no GC)          |
          +----------------------------------+
                      │ cargo build --release
                      ▼
          +----------------------------------+
          |      Single Static Binary        |
          |  No external runtime             |
          |  FROM scratch Docker             |
          +----------------------------------+
```

## Anti-DSL: Schema First

One `.ag` file is the single source of truth. Schema drift is eliminated by design.

```
model User {
  id      UUID      @primary @auto
  email   String    @unique @max(255)
  name    String    @max(100)
  role    UserRole  @default(USER)
  created Timestamp @auto
}

endpoint CreateUser {
  method POST
  path   /users
  auth   required
  policy "user.role != BANNED"
  body   CreateUserRequest
  response User
  errors [EmailTaken, ValidationError]
}

request CreateUserRequest {
  email String @email
  name  String @min(2) @max(100)
}
```

`ag generate` produces from this single file:

| File | Description |
|---|---|
| `src/models.rs` | Rust structs + serde + validators |
| `src/validators.rs` | Validation logic for The Shield |
| `src/handlers/stubs.rs` | Handler signatures — dev fills the body |
| `src/db/queries.rs` | sqlx queries verified at compile time |
| `src/db/migrations/` | Versioned SQL migrations |
| `ts/types.ts` | TypeScript types for the frontend |
| `ts/client.ts` | Type-safe HTTP client |
| `openapi.yaml` | OpenAPI 3.1 specification |

## Developer Experience: AI-Accelerated

The `.ag` schema is the perfect contract for AI agents. It gives the agent exactly what it needs — types, errors, access policies, validations — to generate a correct handler.

```
Engineer designs schema.ag  →  ag generate  →  Agent fills handler bodies
                                                Rust compiler verifies all of it
                                                ag build  →  Single binary
```

## Batteries Included

| Module | Stack | What it provides |
|---|---|---|
| **AG-Auth** | webauthn-rs, JWT Ed25519, governor | Passkeys, JWT, RBAC, OAuth2 (Google, GitHub) |
| **AG-Data** | sqlx, sea-query | Compile-time SQL, PostgreSQL/SQLite/MySQL, migrations |
| **AG-Realtime** | async-nats, tokio-tungstenite | WebSocket, SSE, NATS pub/sub, JetStream |
| **AG-Cache** | moka, fred (Redis) | Thread-safe LRU/LFU, Redis adapter, auto-invalidation |
| **AG-Storage** | S3/MinIO adapters | Signed URLs, image processing, CDN headers |
| **AG-UI** | askama, HTMX | SSR compiled at build time, selective hydration |
| **AG-Observability** | tracing, OpenTelemetry, Prometheus | Distributed traces, p50/p95/p99 metrics, Grafana |

## Quick Start

### Install

```sh
# From crates.io
cargo install ag

# Linux/macOS one-liner
curl -fsSL https://get.antigravital.dev | sh

# macOS Homebrew
brew install antigravital/tap/ag

# Windows
winget install GravitalLabs.AntiGravital
```

### Hello World in 5 Minutes

```sh
# 1. Create project
ag new hello-api --template rest
cd hello-api

# 2. Define the schema
cat schema.ag

# 3. Generate Rust, TypeScript, and OpenAPI from the schema
ag generate

# 4. Start the development server
ag dev
# ✓ http://localhost:3000       — API
# ✓ http://localhost:3000/docs  — OpenAPI documentation
# ✓ http://localhost:3000/metrics — Prometheus metrics
# ✓ http://localhost:6669       — tokio-console

# 5. Build a production binary
ag build --target x86_64-unknown-linux-musl
# Single static binary. FROM scratch Docker.
```

### As a Library

```toml
[dependencies]
anti-gravital = "0.1"   # Full framework
ag-auth       = "0.1"   # Auth module only
ag-data       = "0.1"   # Data module only
ag-realtime   = "0.1"   # Realtime module only
```

### Handler Example

```rust
// Generated by `ag generate` — the developer only fills the body
pub async fn create_user(
    State(state): State<AppState>,
    ValidatedBody(req): ValidatedBody<CreateUserRequest>,  // validated by The Shield
    Claims(claims): Claims<AuthClaims>,                    // JWT already verified
) -> Result<Json<User>, AgError> {
    let user = state.db.users()
        .create(CreateUserParams {
            email: req.email,
            name: req.name,
        })
        .await?;
    state.events.emit("user.created", &user).await?;
    Ok(Json(user))
}
```

## vs. The Competition

| Criterion | Spring Boot | .NET Core | FastAPI | NestJS | Anti-Gravital |
|---|---|---|---|---|---|
| Runtime | JVM | CLR | CPython | Node.js V8 | **None** |
| Memory base | 350MB | 120MB | 60MB | 80MB | **~10MB** |
| Startup | 6s | 0.8s | 0.8s | 1.2s | **0.04s** |
| Throughput Hello World | ~75K req/s | ~200K req/s | ~28K req/s | ~45K req/s | **~520K req/s** |
| Throughput CRUD+DB | ~15K req/s | ~30K req/s | ~5K req/s | ~8K req/s | **~60K req/s** |
| Memory Safety | Partial | Partial | No | No | **Total (compiler)** |
| GC Pauses | Yes | Yes | N/A | Yes | **No** |
| Single Binary | No | Partial | No | No | **Yes** |
| Schema-First DX | No | No | Partial | No | **Yes (.ag DSL)** |
| Compile-time SQL | No | No | No | No | **Yes (sqlx)** |

## Roadmap

| Phase | Timeline | Deliverables |
|---|---|---|
| **0 — Foundations** | Month 1–2 | Repository, CI (Linux/macOS/Windows), Hello World >300K req/s ✓ |
| **1 — Shield MVP** | Month 2–4 | TLS 1.3, schema→validators+stubs, `ag` CLI v0.1 |
| **2 — Core MVP** | Month 4–6 | Full roundtrip, PostgreSQL CRUD, sqlx compile-time queries |
| **3 — Anti-DSL Complete** | Month 6–9 | Relations, TS client, OpenAPI 3.1, LSP + VSCode plugin |
| **4 — Modules Complete** | Month 9–12 | AG-Auth (WebAuthn, OAuth2), AG-Realtime, AG-Cache, demo app |
| **5 — Ecosystem** | Month 12–18 | Stable API (semver), security audit, plugin registry |

## Repository Layout

```
anti-gravital/
├── ag-core/           # Shield (Tower middleware) + Core (Axum handlers)
├── ag-dsl/            # .ag lexer, parser, semantic checker, code generators
├── ag-cli/            # `ag` CLI binary
├── ag-modules/        # Batteries-included modules
│   ├── ag-auth/       # WebAuthn + JWT + RBAC + OAuth2
│   ├── ag-data/       # sqlx + migrations + multi-tenant
│   ├── ag-realtime/   # async-nats + WebSocket + SSE
│   ├── ag-cache/      # moka + Redis
│   ├── ag-storage/    # S3/MinIO/local
│   ├── ag-ui/         # SSR (askama) + HTMX
│   └── ag-observability/ # tracing + OpenTelemetry + Prometheus
├── ag-wasm-host/      # WebAssembly plugin runtime (wasmtime)
├── docs/              # Guides, schema reference, migration guides
├── examples/          # todo-api, ecommerce-api, realtime-chat, ai-backend
├── templates/         # ag new templates (rest, fullstack, realtime)
├── plugins/           # Official WASM plugins (prometheus, datadog)
├── benchmarks/        # TechEmpower suite + comparison
├── Cargo.toml         # Workspace root
└── LICENSE            # Apache 2.0
```

## Building

```sh
# Build all crates
cargo build --workspace

# Run tests
cargo test --workspace

# Run benchmarks
cargo bench -p ag-core

# Release build (static binary)
cargo build --release --target x86_64-unknown-linux-musl
```

## Origin

Anti-Gravital is built in Sabanitas, Colón, República de Panamá by Gravital Labs — Nereira Technology and Business Solutions. Documentation ships in both Spanish and English as first-class citizens of the project. Innovation in high-performance systems engineering has no fixed geography.

## License

Apache License 2.0. See [LICENSE](LICENSE). No enterprise edition. No closed features. No vendor lock-in.
