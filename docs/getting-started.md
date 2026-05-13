# Getting Started

This guide walks through installing Anti-Gravital, creating your first project, and understanding the development workflow.

## Prerequisites

- **Rust 1.75 or newer** — `rustup update stable`

That is the only prerequisite. Anti-Gravital compiles to a single static binary with no external runtime.

Verify:

```sh
rustc --version   # rustc 1.75.0 or newer
cargo --version   # cargo 1.75.0 or newer
```

## Install the CLI

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

Or build from source:

```sh
git clone https://github.com/gravital-labs/anti-gravital
cd anti-gravital
cargo install --path ag-cli
ag --version
```

## Create a Project

```sh
ag new my-api --template rest
cd my-api
```

This creates:

```
my-api/
  schema.ag          # API contract — the single source of truth
  src/
    handlers/        # Handler implementations (filled in by the developer)
  .gitignore
```

## Define Your Schema

Open `schema.ag`:

```
model User {
  id      UUID      @primary @auto
  email   String    @unique @max(255)
  name    String    @min(1) @max(100)
  created Timestamp @auto
}

endpoint ListUsers {
  method   GET
  path     /users
  auth     required
  response User
}

endpoint CreateUser {
  method   POST
  path     /users
  auth     required
  body     CreateUserRequest
  response User
  errors   [EmailTaken, ValidationError]
}

request CreateUserRequest {
  email String @email
  name  String @min(2) @max(100)
}
```

## Generate Code

```sh
ag generate
```

`ag generate` reads `schema.ag` and writes:

| File | Description |
|---|---|
| `src/models.rs` | Rust structs with serde derives |
| `src/validators.rs` | Shield validation logic |
| `src/handlers/stubs.rs` | Handler signatures — fill in the body |
| `src/db/queries.rs` | sqlx queries verified at compile time |
| `src/db/migrations/` | Versioned SQL migrations |
| `ts/types.ts` | TypeScript types for the frontend |
| `ts/client.ts` | Type-safe HTTP client |
| `openapi.yaml` | OpenAPI 3.1 specification |

## Implement a Handler

Open the generated stub and fill in the business logic:

```rust
// src/handlers/users.rs — generated stub, developer fills the body

pub async fn create_user(
    State(state): State<AppState>,
    ValidatedBody(req): ValidatedBody<CreateUserRequest>,  // validated by The Shield
    Claims(claims): Claims<AuthClaims>,                    // JWT already verified
) -> Result<Json<User>, AgError> {
    // Only this body is written by the developer:
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

The Shield has already verified the JWT and validated the request body before this function is called.

## Start the Development Server

```sh
ag dev
```

Output:

```
Anti-Gravital server starting addr=127.0.0.1:3000
listening addr=127.0.0.1:3000
```

Endpoints available:

| URL | Description |
|---|---|
| `http://localhost:3000` | Your API |
| `http://localhost:3000/health` | Health check |
| `http://localhost:3000/metrics` | Prometheus metrics |
| `http://localhost:3000/docs` | OpenAPI documentation (Phase 1) |
| `http://localhost:6669` | tokio-console async task inspector |

## Build for Production

```sh
ag build --target x86_64-unknown-linux-musl
```

This produces a single static binary with zero runtime dependencies:

```sh
file target/release/my-api
# my-api: ELF 64-bit LSB executable, statically linked

ldd target/release/my-api
# statically linked (no shared libraries)
```

The Dockerfile for production:

```dockerfile
FROM scratch
COPY target/x86_64-unknown-linux-musl/release/my-api /app
ENTRYPOINT ["/app"]
```

## Project Structure

```
my-api/
  schema.ag              # API contract (the only file you need to edit)
  src/
    models.rs            # Generated — do not edit
    validators.rs        # Generated — do not edit
    handlers/
      stubs.rs           # Generated stubs
      users.rs           # Your implementations
    db/
      queries.rs         # Generated sqlx queries
      migrations/        # Generated SQL migrations
  ts/
    types.ts             # Generated TypeScript types
    client.ts            # Generated HTTP client
  openapi.yaml           # Generated OpenAPI 3.1 spec
```

## Next Steps

- Read the [Schema Reference](schema-reference.md) to learn all field types, directives, and endpoint options.
- Read the [Architecture](architecture.md) to understand how The Shield and The Core interact.
- Check the `examples/` directory for complete working applications (todo-api, ecommerce-api, realtime-chat, ai-backend).
