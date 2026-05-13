# Architecture

Anti-Gravital v3.0 — Blueprint Técnico, Gravital Labs

Anti-Gravital is structured as two cooperating layers — **The Shield** and **The Core** — running in a single Rust binary on a single Tokio async runtime. There is no inter-process communication, no shared memory, no CGO bridge, and no serialization cost between layers. The communication between Shield and Core is an ordinary Rust function call: zero measurable overhead.

## The Dual: One Process, Two Layers

```
                     Incoming Requests
                            |
          +-----------------v------------------+
          |       ANTI-GRAVITAL RUNTIME        |
          |           (~10MB base)             |
          |                                    |
          |  THE SHIELD (Capa A)               |
          |  Tower middleware pipeline         |
          |                                    |
          |  SetRequestIdLayer                 |  x-request-id UUID
          |  TraceLayer                        |  structured HTTP spans
          |  TimeoutLayer                      |  per-request deadline
          |  CompressionLayer                  |  gzip/deflate/br
          |  CorsLayer                         |  CORS + preflight
          |                                    |
          |  ShieldLayer (Phase 1+)            |
          |  ├─ TLS 1.3 (rustls)              |
          |  ├─ JWT Ed25519 verification       |
          |  ├─ Schema validation (.ag)        |
          |  ├─ Rate limiting (governor)       |
          |  ├─ RBAC guards                    |
          |  └─ CSRF protection               |
          |       │                            |
          |       │  Rust function call (0ns)  |
          |       ▼                            |
          |  THE CORE (Capa B)                 |
          |  Axum router + handlers            |
          |                                    |
          |  GET  /health                      |
          |  GET  /metrics                     |
          |  ... application routes ...        |
          |                                    |
          |  Business logic (Tokio tasks)      |
          |  AG-Data (sqlx, compile-time SQL)  |
          |  AG-Realtime (async-nats, WS, SSE) |
          +-----------------+------------------+
                            |
                     HTTP Response
                            │
                    cargo build --release
                            │
          +-----------------v------------------+
          |        Single Static Binary        |
          |  No external runtime               |
          |  FROM scratch Docker               |
          +------------------------------------+
```

## The Shield (Capa A — Tower Middleware)

Source: `ag-core/src/shield/`

The Shield is the only layer that touches untrusted network data. It is built as a Tower `Layer`, the same composable middleware model that Axum uses internally. The result is a pipeline where each layer has a single responsibility and composes at compile time.

### Current Tower Stack

```rust
let middleware = ServiceBuilder::new()
    .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
    .layer(TraceLayer::new_for_http())
    .layer(TimeoutLayer::new(config.request_timeout))
    .layer(CompressionLayer::new())
    .layer(CorsLayer::permissive()); // restricted in production
```

### Phase 1 Additions (ShieldLayer)

The `ShieldLayer` / `ShieldService` wraps the Core and will enforce:

1. **Schema validation** — Request headers, path parameters, and body structure are checked against compiled `.ag` contracts. A malformed request returns `400 Bad Request` before any handler code executes.
2. **JWT verification** — `Authorization: Bearer <token>` is verified with Ed25519 via the `ring` crate. Verified claims are injected into the request extensions as `RequestContext`.
3. **Rate limiting** — Token-bucket rate limiting per client IP via `governor`. `429 Too Many Requests` is returned before any I/O.
4. **RBAC guards** — Policy expressions from the `.ag` schema (e.g. `"user.role != BANNED"`) are evaluated against the verified claims.

The Shield allocates zero bytes on the heap for well-formed, authenticated requests.

## The Core (Capa B — Axum Handlers)

Source: `ag-core/src/core/`

The Core contains the application handler layer. It consists of:

### `AgError` (`core/error.rs`)

A typed error enum implementing `IntoResponse`. All handler errors flow through this type and serialize to a consistent JSON envelope:

```json
{
  "error": {
    "code": "NOT_FOUND",
    "message": "user not found"
  }
}
```

| Variant | HTTP Status |
|---|---|
| `NotFound` | 404 |
| `BadRequest` | 400 |
| `Validation` | 422 |
| `Unauthorized` | 401 |
| `Forbidden` | 403 |
| `Internal` | 500 |

### `ValidatedBody<T>` (`core/context.rs`)

An Axum extractor that enforces `Content-Type: application/json` and deserializes the body into `T: DeserializeOwned`. Returns `AgError::BadRequest` on malformed JSON. In Phase 1+, validation against the `.ag` schema runs here inside the Shield before the extractor is invoked.

### `RequestContext` (`core/context.rs`)

Metadata set by the Shield and consumed by handlers:

```rust
pub struct RequestContext {
    pub request_id: Option<String>, // from x-request-id
    pub subject: Option<String>,    // from verified JWT sub claim
    pub client_ip: Option<IpAddr>,  // after proxy header resolution
}
```

Available via `axum::extract::Extension<RequestContext>`.

### Generated Handler Signature

`ag generate` produces handler stubs that the developer fills in:

```rust
pub async fn create_user(
    State(state): State<AppState>,
    ValidatedBody(req): ValidatedBody<CreateUserRequest>,  // validated by The Shield
    Claims(claims): Claims<AuthClaims>,                    // JWT already verified
) -> Result<Json<User>, AgError> {
    // Developer writes only this body
    let user = state.db.users()
        .create(CreateUserParams { email: req.email, name: req.name })
        .await?;
    state.events.emit("user.created", &user).await?;
    Ok(Json(user))
}
```

## The Anti-DSL Compiler

Source: `ag-dsl/`

The compiler transforms `.ag` schema files into generated source code:

```
schema.ag
  └─ Lexer     (ag-dsl/src/lexer.rs)        tokenization
      └─ Parser (ag-dsl/src/parser.rs)       AST construction
          └─ Semantic (ag-dsl/src/semantic.rs)  type checking, cross-references
              └─ Codegen (ag-dsl/src/codegen/)
                  ├─ rust.rs       → src/models.rs, src/validators.rs, src/handlers/stubs.rs
                  ├─ sqlx.rs       → src/db/queries.rs, src/db/migrations/
                  ├─ typescript.rs → ts/types.ts, ts/client.ts
                  └─ openapi.rs    → openapi.yaml
```

The `ag-dsl::compile(src: &str)` function runs the full pipeline and returns a validated `SchemaFile` AST. `ag generate --write` calls `codegen::generate_all` and writes output files.

## Modules (Batteries Included)

Each module is an independent crate in `ag-modules/`. They integrate via `AppState` and are opt-in.

| Module | Stack | Responsibility |
|---|---|---|
| `ag-auth` | webauthn-rs, ring (Ed25519), governor | Passkeys, JWT, RBAC, OAuth2 |
| `ag-data` | sqlx, sea-query | Compile-time SQL, PostgreSQL/SQLite/MySQL, migrations |
| `ag-realtime` | async-nats, tokio-tungstenite | WebSocket, SSE, NATS pub/sub, JetStream |
| `ag-cache` | moka, fred (Redis) | Thread-safe LRU/LFU in-memory, Redis adapter |
| `ag-storage` | aws-sdk-s3, MinIO | Signed URLs, image processing, CDN headers |
| `ag-ui` | askama | Templates compiled at build time, SSR, HTMX integration |
| `ag-observability` | tracing, opentelemetry, metrics | Distributed traces, Prometheus metrics, Grafana |

## AppState

Source: `ag-core/src/app.rs`

`AppState` is `Clone` (cheap, `Arc`-backed) and is passed to every handler via `axum::extract::State<AppState>`. In Phase 2+, the generated `AppState` from `ag generate` adds typed fields for each enabled module:

```rust
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<ServerConfig>,
    pub started_at: Instant,
    // Added by ag generate in Phase 2+:
    // pub db: ag_data::Pool,
    // pub events: ag_realtime::Client,
    // pub cache: ag_cache::Cache,
}
```

## Request Lifecycle

1. Tokio's async I/O accepts the TCP connection.
2. Hyper parses the HTTP request.
3. `SetRequestIdLayer` attaches an `x-request-id` UUID.
4. `TraceLayer` opens a tracing span.
5. `TimeoutLayer` arms a per-request deadline.
6. `ShieldLayer` (Phase 1+): validates schema contract, verifies JWT, enforces rate limits and RBAC. Injects `RequestContext` into the extension map.
7. Axum router matches path and method, selects the handler function.
8. Handler executes as a `Future` on the Tokio thread pool. It may `await` database queries, cache lookups, or external calls.
9. Handler returns `Result<impl IntoResponse, AgError>`.
10. `TraceLayer` records the response status and latency.
11. `CompressionLayer` compresses the body if the client accepts it.
12. HTTP response is sent to the client.

Steps 1–7 and 9–12 target zero heap allocation for well-formed authenticated requests (Phase 1 goal). Step 8 allocates only what the application logic requires.

## Why Pure Rust (Appendix B Summary)

The v1.0 blueprint proposed a POSIX shared memory ring buffer ("Memory Bus") connecting Rust and Go runtimes. The latency argument was correct (shared memory ≈ 500ns vs 500µs for HTTP local), but the operational cost was ignored: fragmented debugging and profiling, broken cross-compilation, dual toolchain in CI, and impossible unified stack traces.

With a pure Rust implementation, Shield → Core communication is an ordinary function call: **zero overhead**. The profiler covers the entire application. Cross-compilation is `cargo build --target`. The stack trace is one runtime. The bottleneck is always the database, never intra-process communication.

## Configuration

Source: `ag-core/src/config.rs`

```rust
ServerConfig::builder()
    .addr("0.0.0.0:8080".parse().unwrap())
    .request_timeout(Duration::from_secs(10))
    .max_body_bytes(1 * 1024 * 1024)
    .cors_origin("https://app.example.com")
    .build()
```
