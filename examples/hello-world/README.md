# Hello World

The simplest possible Anti-Gravital application: a single `GET /health` endpoint that returns the server status as JSON.

## Requirements

- `ag` CLI installed (`cargo install --path ../../ag-cli` from the repo root)

## Run

```sh
ag new hello-world --template rest
cd hello-world
ag dev
```

In a second terminal:

```sh
curl -s http://localhost:3000/health | jq .
```

Expected response:

```json
{
  "status": "ok",
  "version": "0.1.0",
  "uptime": 0
}
```

## What This Demonstrates

- `ag new` scaffolds a project with a `schema.ag` file and a Rust handler stub.
- `schema.ag` defines the `HealthCheck` model and the `Health` endpoint.
- `ag dev` compiles and starts the Axum server — no separate build step.
- The Shield layer validates the request (no body expected for GET /health).
- The Core dispatches it to the `health` handler via the Axum router.
- The handler returns `Json<HealthCheck>` which Axum serializes to the client.
- The entire stack — from TCP accept to HTTP response — runs in a single Rust binary.
