# Getting Started

This guide walks through installing Anti-Gravital, creating your first project,
and understanding the development workflow.

## Prerequisites

- **Rust 1.75 or newer** — `rustup update stable`
- **Go 1.21 or newer** — https://go.dev/dl/
- **Linux or macOS** — Windows support is planned for Phase 4

Verify your installations:

```sh
rustc --version   # rustc 1.75.0 or newer
cargo --version   # cargo 1.75.0 or newer
go version        # go1.21.0 or newer
```

## Install the CLI

Clone the repository and install the `ag` binary:

```sh
git clone https://github.com/angelnereira/anti-gravital
cd anti-gravital
cargo install --path ag-cli
```

Verify the installation:

```sh
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
  schema.ag           # API contract definition
  go.mod              # Go module file
  src/
    handlers/
      health.go       # Health check handler stub
  .gitignore
```

## Define Your Schema

Open `schema.ag` and define your models and endpoints:

```
@version 1.0
@namespace api

model User {
    id       UUID      @primary @auto
    email    String    @unique @format(email) @max(255)
    name     String    @min(1) @max(100)
    created  Timestamp @auto
}

endpoint ListUsers {
    method    GET
    path      /users
    auth      required
    response  User[]
}

endpoint CreateUser {
    method    POST
    path      /users
    body      CreateUserRequest
    response  User
}

request CreateUserRequest {
    email  String  @email
    name   String  @min(1) @max(100)
}
```

## Generate Code

```sh
ag generate
```

Once the Anti-DSL parser is complete (Phase 3), this command produces:

- `src/go/models.go` — Go structs for User and CreateUserRequest
- `src/go/handlers_stubs.go` — Handler stubs for ListUsers and CreateUser
- `src/go/queries.sql.go` — Type-safe sqlc query functions
- `src/ts/types.ts` — TypeScript types for the frontend
- `openapi.yaml` — OpenAPI 3.1 documentation

## Implement a Handler

Open the generated stub and fill in the business logic:

```go
// src/handlers/users.go

package handlers

import "github.com/gravital-labs/anti-gravital/ag-runtime/brain"

type UserHandler struct {
    // db is the AG-Data database handle (generated from schema.ag)
    // db *agdata.Database
}

func (h *UserHandler) CreateUser(ctx brain.Context) error {
    // The request body has already been validated by the Shield layer.
    // ctx.Body() contains the raw bytes; the generated decoder will
    // deserialize them into a CreateUserRequest struct.
    return ctx.JSON(201, map[string]string{
        "id":    "00000000-0000-0000-0000-000000000001",
        "email": "example@example.com",
        "name":  "Example User",
    })
}
```

## Start the Development Server

```sh
ag dev
```

Output:

```
Starting development server on http://127.0.0.1:3000
Documentation: http://127.0.0.1:3000/docs
Metrics:       http://127.0.0.1:3000/metrics
Press Ctrl+C to stop
Shield starting addr=127.0.0.1:3000 max_connections=65535 tls=false
Memory bus connected bus_capacity=8190
```

## Build for Production

```sh
ag build --target x86_64-unknown-linux-musl
```

This produces a single static binary with zero runtime dependencies:

```sh
ls -lh dist/ag
# -rwxr-xr-x 1 user group 8.2M dist/ag

ldd dist/ag
# statically linked (no shared libraries)

./dist/ag --version
# ag 0.1.0
```

Deploy the binary to any Linux server:

```sh
scp dist/ag user@server:/opt/my-api/bin/
ssh user@server '/opt/my-api/bin/ag'
```

## Project Structure Reference

```
my-api/
  schema.ag                   # API contract (edit this)
  go.mod                      # Go module
  src/
    handlers/                 # Your handler implementations
      health.go
      users.go
    middleware/               # Optional custom middleware
  dist/                       # Build output (gitignored)
```

## Next Steps

- Read the [Schema Reference](schema-reference.md) to learn all the field types
  and endpoint options.
- Read the [Architecture](architecture.md) to understand how the Shield, Brain,
  and Memory Bus interact.
- Check the `examples/` directory for complete working applications.
