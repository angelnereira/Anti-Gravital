# Architecture

Anti-Gravital is structured as two cooperating layers — The Shield and The Core — running in a single Rust binary on a single Tokio async runtime. There is no inter-process communication, no shared memory segment, and no serialization cost between layers.

## Layer Overview

```
                     Incoming Requests
                            |
              +-------------v--------------+
              |        Tower Stack          |
              |                             |
              |  SetRequestIdLayer          |  adds x-request-id header
              |  TraceLayer                 |  structured HTTP tracing
              |  TimeoutLayer               |  per-request deadline
              |  CompressionLayer           |  gzip/deflate/br response
              |  CorsLayer                  |  preflight + CORS headers
              |                             |
              |  ShieldLayer (Phase 1+)     |  schema validation
              |  ag-core/src/shield/        |  JWT / Ed25519 verify
              |                             |  rate limiting
              +-------------+---------------+
                            |
              +-------------v---------------+
              |        Axum Router          |
              |                             |
              |  GET  /health               |  ag-core/src/app.rs
              |  GET  /metrics              |
              |  ... application routes ... |  ag-core/src/core/
              +-------------+---------------+
                            |
                     HTTP Response
```

All layers share the same OS thread pool managed by Tokio. Middleware composes at compile time via Tower's `Layer` trait; the runtime cost is a single async await per middleware.

## The Shield (Tower Layer)

Source: `ag-core/src/shield/`

The Shield is a Tower `Layer` implementation. It wraps any inner `Service<Request>` and intercepts every request before it reaches the Core.

Current implementation (`ShieldLayer`, `ShieldService`) is a transparent pass-through that establishes the stable public interface. Phase 1 will add:

1. **Schema validation**: Request headers, path parameters, and body structure are checked against the compiled `.ag` contract. A `400 Bad Request` is returned immediately — the handler never runs.
2. **JWT verification**: The `Authorization: Bearer <token>` header is verified using Ed25519. The verified claims are stored in the request extension map as `RequestContext`.
3. **Rate limiting**: Token-bucket rate limiting per client IP. Exceeding the limit returns `429 Too Many Requests` before any I/O.

The Shield never allocates on the hot path for well-formed requests. All validation is done against pre-compiled contract data.

```rust
// Tower Layer composition
let middleware = ServiceBuilder::new()
    .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
    .layer(TraceLayer::new_for_http())
    .layer(TimeoutLayer::new(config.request_timeout))
    .layer(CompressionLayer::new())
    .layer(CorsLayer::permissive());
```

## The Core (Axum Handlers)

Source: `ag-core/src/core/`

The Core is the application handler layer. It consists of:

### `AgError` (`core/error.rs`)

A typed error enum that implements `IntoResponse`. All handler errors flow through this type and are serialized to a consistent JSON envelope:

```json
{
  "error": {
    "code": "NOT_FOUND",
    "message": "user not found"
  }
}
```

Status code mapping:

| Error variant     | HTTP status |
|-------------------|-------------|
| `NotFound`        | 404         |
| `BadRequest`      | 400         |
| `Validation`      | 422         |
| `Unauthorized`    | 401         |
| `Forbidden`       | 403         |
| `Internal`        | 500         |

### `ValidatedBody<T>` (`core/context.rs`)

An Axum extractor that enforces `Content-Type: application/json` and deserializes the body into `T: DeserializeOwned`. Returns `AgError::BadRequest` on malformed JSON and `AgError::Validation` on an empty body.

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

### `with_shield` (`core/router.rs`)

A convenience function that wraps a sub-router with `ShieldLayer`:

```rust
let protected = with_shield(
    Router::new()
        .route("/api/users", get(list_users))
        .route("/api/users/:id", get(get_user))
);
```

## The Anti-DSL Compiler

Source: `ag-dsl/`

The DSL compiler transforms `.ag` schema files into generated source code. The pipeline is:

```
.ag source
    └─> Lexer (ag-dsl/src/lexer.rs)        tokenization
        └─> Parser (ag-dsl/src/parser.rs)   AST construction
            └─> Semantic (ag-dsl/src/semantic.rs)  type checking
                └─> Codegen (ag-dsl/src/codegen/)  code emission
                    ├─> rust.rs    →  src/generated/models.rs
                    ├─> typescript.rs → src/generated/models.ts
                    └─> openapi.rs →  openapi.json
```

The `ag-dsl::compile(src: &str)` function runs the full pipeline and returns a validated `SchemaFile` AST. The `ag generate --write` CLI command calls `codegen::generate_all` and writes the output files.

## AppState

Source: `ag-core/src/app.rs`

`AppState` is cloned per request (cheap, `Arc`-backed). It carries:

- `config: Arc<ServerConfig>` — server configuration
- `started_at: Instant` — for uptime calculation in `/health`

Application-specific state (database pools, cache handles) is added to `AppState` and accessed in handlers via `axum::extract::State<AppState>`.

## Request Lifecycle

For a well-formed authenticated request:

1. Tokio's async I/O layer accepts the TCP connection.
2. Hyper parses the HTTP request.
3. `SetRequestIdLayer` attaches an `x-request-id` UUID.
4. `TraceLayer` opens a tracing span for the request.
5. `TimeoutLayer` arms a per-request deadline.
6. `ShieldLayer` (Phase 1+) validates the schema contract and verifies the JWT. Injects `RequestContext` into the extension map.
7. The Axum router matches the path and method, selects the handler.
8. The handler function executes as a `Future` on the Tokio thread pool. It may `await` database queries, cache lookups, or external calls.
9. The handler returns `Result<impl IntoResponse, AgError>`.
10. `TraceLayer` records the response status and latency.
11. `CompressionLayer` compresses the body if the client accepts it.
12. The HTTP response is written back to the client.

Steps 1–7 and 9–12 allocate zero bytes on the heap for well-formed requests (Phase 1 target). Step 8 allocates only what the application logic requires.

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

## Code Generation

The Anti-DSL compiler (`ag-dsl`) reads `.ag` schema files and generates:

- **Rust** (`src/generated/models.rs`): Serde-annotated structs derived from `model` declarations.
- **TypeScript** (`src/generated/models.ts`): TypeScript interfaces for frontend consumers.
- **OpenAPI 3.1** (`openapi.json`): Machine-readable API documentation, including request/response schemas for all declared endpoints.

The `.ag` file is the single source of truth for the API contract. Type drift between frontend and backend becomes a compile error rather than a runtime bug.
