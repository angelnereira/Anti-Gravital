# Hello World

The simplest possible Anti-Gravital application: a single GET /health endpoint
that returns the server status as JSON.

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

- The `ag new` scaffold sets up the project structure.
- The `schema.ag` file defines the HealthCheck model and Health endpoint.
- The handler in `src/handlers/health.go` implements the endpoint logic.
- `ag dev` starts the development server without requiring a separate build step.
- The Shield layer validates the request (no body expected for GET /health).
- The Brain dispatches it to the HealthHandler goroutine pool.
- The response travels back through the Memory Bus to the Rust layer.
- The Rust layer serializes and sends the HTTP response.
