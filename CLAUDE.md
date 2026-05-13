# Anti-Gravital — Claude Code Project Context

## What This Is

Anti-Gravital is a high-performance web framework written in pure Rust, built by Gravital Labs
(Nereira Technology and Business Solutions, Sabanitas, Colon, Panama). It compiles to a single
static binary with no external runtime dependencies.

Target metrics: ~520K req/s throughput, ~10MB base memory, 0.04s startup.

The canonical reference for all architecture and design decisions is `docs/BLUEPRINT_v3.md`.

## Repository Layout

```
ag-core/          Shield (Tower middleware) + Core (Axum handlers)
ag-dsl/           .ag DSL compiler: lexer → parser → AST → semantic → codegen
ag-cli/           `ag` binary: new, generate, dev, build
ag-modules/
  ag-auth/        WebAuthn + JWT Ed25519 + RBAC (stub)
  ag-data/        sqlx + migrations (stub)
  ag-realtime/    async-nats + WebSocket (stub)
  ag-cache/       moka + Redis (stub)
  ag-storage/     S3/MinIO (stub)
  ag-ui/          SSR + HTMX (stub)
  ag-observability/ tracing + OpenTelemetry (stub)
ag-wasm/          wasmtime plugin host
benchmarks/       Criterion benchmarks
docs/             Architecture, getting-started, schema-reference, BLUEPRINT_v3.md
examples/         hello-world
templates/        ag new templates
```

## Non-Negotiable Rules

- **Pure Rust only.** No Go. No Node. No Python. No second runtime anywhere in the codebase.
- **No emojis** in any file, commit message, or output.
- **English** for all code, comments, and technical documentation.
- **No AI references** in branch names.

## Development

```sh
cargo build --workspace          # build everything
cargo test --workspace           # run all tests
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all
```

CI runs on Linux, macOS, and Windows. All three must pass.

## The .ag DSL — v3.0 Syntax

The schema file is the single source of truth. `ag generate` produces all artifacts from it.

```ag
enum UserRole {
  USER
  ADMIN
}

model User {
  id      UUID      @primary @auto
  email   String    @unique @max(255)
  name    String    @max(100)
  role    UserRole  @default(USER)
  created Timestamp @auto
}

request CreateUserRequest {
  email String @email
  name  String @min(2) @max(100)
}

endpoint CreateUser {
  method   POST
  path     /users
  auth     required
  policy   "user.role != BANNED"
  body     CreateUserRequest
  response User
  errors   [EmailTaken, ValidationError]
}
```

Critical syntax rules:
- Field declaration: `name Type @directives` — NO colon between name and type.
- Endpoint declaration: named block syntax (`endpoint Name { ... }`), NOT inline (`endpoint GET /path`).
- `request` keyword is separate from `model` — used for request body types only.
- `?` suffix on type makes field optional.
- Comments: `# line comment`.

## Architecture: The Dual

Two conceptual layers in one Rust process — zero IPC, zero FFI:

- **The Shield (Capa A)**: Tower middleware pipeline — TLS 1.3 (rustls), JWT Ed25519 (ring),
  schema validation, rate limiting (governor), RBAC, CORS/CSRF.
- **The Core (Capa B)**: Axum router + handlers, Tokio tasks, AG-Data (sqlx), AG-Realtime (async-nats).

Shield calls Core via ordinary Rust function calls — zero overhead.

## ag-dsl Crate Structure

- `lexer.rs` — tokenizes .ag source, produces `Vec<Lexeme>`
- `parser.rs` — recursive descent, produces `SchemaFile` AST
- `ast.rs` — AST types: `SchemaFile`, `ModelDef`, `RequestDef`, `EnumDef`, `EndpointDef`, `FieldDef`, `FieldType`, `Directive`
- `semantic.rs` — validates field types, body/response refs, detects duplicates
- `codegen/rust.rs` — generates `src/models.rs`
- `codegen/typescript.rs` — generates `ts/types.ts`
- `codegen/openapi.rs` — generates `openapi.yaml`
- `lib.rs` — public `compile(src: &str) -> Result<SchemaFile, DslError>`

## ag-cli Commands

- `ag new <name> --template rest` — scaffold new project
- `ag generate` — read `schema.ag`, write generated files (default, no flag needed)
- `ag generate --dry-run` — preview without writing
- `ag dev` — run with hot reload (planned)
- `ag build` — cargo build --release (planned)

## Key Dependencies

```toml
tokio         = { features = ["full"] }
axum          = "0.8"
tower         = "0.5"
tower-http    = { features = ["trace", "cors"] }
serde         = { features = ["derive"] }
serde_json    = "1"
ring          = "0.17"
rustls        = "0.23"
governor      = "0.6"
sqlx          = { features = ["postgres", "runtime-tokio", "macros"] }
async-nats    = "0.35"
moka          = { features = ["future"] }
tracing       = "0.1"
opentelemetry = "0.24"
```

## Roadmap Phase Status

| Phase | Description | Status |
|---|---|---|
| Phase 0 | Foundations: repo, CI, ag-core, ag-dsl v3.0 | Done |
| Phase 1 | Shield MVP: TLS, JWT, real middleware | In progress |
| Phase 2 | Core MVP: full DB roundtrip | Planned |
| Phase 3 | Anti-DSL complete: relations, LSP, OpenAPI 3.1 | Planned |
| Phase 4 | Modules: ag-auth, ag-realtime, ag-cache, ag-observability | Planned |
| Phase 5 | Ecosystem: stable API, security audit, TechEmpower | Planned |
