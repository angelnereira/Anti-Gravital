# Architecture

Anti-Gravital is structured as three cooperating layers called The Triad.
Each layer has a single, well-defined responsibility. The layers communicate
through the Unified Memory Bus — a POSIX shared memory ring buffer that avoids
serialization overhead and system call cost for the common path.

## The Triad

```
                     Incoming Requests
                            |
                     +------v-------+
                     |  The Shield  |   Rust / Tokio
                     |              |   - TLS 1.3 termination (rustls)
                     |  HTTP/TLS    |   - HTTP/1.1 and HTTP/2 parsing (hyper)
                     |  Validation  |   - Schema validation (ag-generated)
                     |  Auth Verify |   - JWT verification (Ed25519, ring)
                     |  Rate Limit  |   - Rate limiting (token bucket, atomic)
                     +------+-------+
                            |
                     Shared Memory Bus
                     (64 MB, 8190 slots)
                            |
                     +------v-------+
                     |  The Brain   |   Go Runtime
                     |              |   - Goroutine worker pool
                     |  Route Match |   - Business logic dispatch
                     |  Handlers    |   - ag-data (sqlc + pgx)
                     |  Modules     |   - ag-realtime (NATS)
                     |              |   - ag-cache, ag-auth, ag-storage
                     +------+-------+
                            |
                     Responses written back
                     into bus slots
```

## The Shield (Rust)

Source: `ag-core/src/shield/`

The Shield is built on Tokio and Hyper. Every incoming TCP connection is
handled by Tokio's async I/O runtime. TLS is terminated by rustls before any
application code runs.

For each request:
1. The path and headers are parsed without copying the body.
2. The schema validator checks the request against the compiled `.ag` contract.
3. If the endpoint requires authentication, the JWT is verified in Rust using
   ring's Ed25519 implementation. The verified claims are placed into the slot.
4. A ring buffer slot is claimed via a CAS loop (see below).
5. The validated request data is written into the slot.
6. The slot state is set to READY with a Release memory store.
7. The Shield spin-waits (or futex-waits) for the response state to become DONE.
8. The response is read and returned to the client.

Steps 1-4 and 7-8 incur no heap allocation on the hot path.

## The Brain (Go)

Source: `ag-runtime/brain/` and `ag-runtime/bus/`

The Brain runs a pool of goroutines. Each goroutine runs a tight loop:

1. Call `ClaimReadSlot()` — CAS scan for a READY slot.
2. If found, transition state to READING.
3. Decode the request fields from shared memory.
4. Dispatch to the matching `HandlerFunc` via the Router.
5. The handler executes business logic (database queries, event publishing, etc.).
6. Write the response into the slot's response area.
7. Set response state to DONE.

The Brain is where application developers write their code. A handler function
has this signature:

```go
func (h *UserHandler) CreateUser(ctx brain.Context) error {
    // ctx.Body()  — request body bytes
    // ctx.Auth()  — verified JWT claims from Rust layer
    // ctx.JSON()  — write response
    return ctx.JSON(201, user)
}
```

## The Memory Bus

Source: `ag-core/src/bus/` (Rust) and `ag-runtime/bus/` (Go)

The bus is a 64 MB POSIX shared memory segment divided into a header and 8190
fixed-size slots of 8192 bytes each.

### Slot Layout

```
Bytes 0..4095     Request area
  [0]             State (u8, atomic)
  [1]             HTTP method code (u8)
  [2..3]          Path length (u16 LE)
  [4..7]          Body length (u32 LE)
  [8..15]         Request ID (u64 LE)
  [16..63]        Reserved
  [64..575]       Auth claims (JWT payload, up to 512 bytes)
  [576..1087]     Path (up to 512 bytes)
  [1088..4095]    Request body payload (up to 3008 bytes)

Bytes 4096..8191  Response area
  [4096]          Response state (u8, atomic)
  [4097]          Reserved
  [4098..4099]    HTTP status code (u16 LE)
  [4100..4103]    Response body length (u32 LE)
  [4104..4159]    Reserved
  [4160..8191]    Response body (up to 4032 bytes)
```

### State Machine

```
        Rust CAS                Rust Release
Empty ──────────> Writing ──────────────> Ready
                                             |
                                   Go CAS   |
                              Reading <──────┘
                                 |
                       Go Release|
                            Done <──────────────
                             |
                    Rust Release (after reading)
                         Empty <───────────────
```

All state transitions use atomic operations with explicit memory ordering:
- WRITING claim: `compare_exchange(Empty, Writing, Acquire, Relaxed)`
- READY signal: `store(Ready, Release)` after all data writes
- READING claim: `compare_exchange(Ready, Reading, Acquire, Relaxed)`
- DONE signal: `store(Done, Release)` after all response writes
- EMPTY release: `store(Empty, Release)` after Rust reads response

### Why Not gRPC / HTTP Localhost?

| Method               | Typical Latency | Notes                                      |
|----------------------|-----------------|--------------------------------------------|
| HTTP localhost       | 200-500 µs      | Full TCP stack, kernel round-trip          |
| gRPC localhost       | 50-200 µs       | Protocol Buffers serialization, TCP        |
| Unix domain socket   | 30-100 µs       | Avoids TCP overhead, still kernel-mediated |
| CGO direct call      | 1-10 µs         | Function call overhead, stack switching    |
| POSIX shared memory  | 100-500 ns      | Cache-line coherence only, no kernel       |

The shared memory approach is the only one where communication between the Rust
layer and Go layer fits within the budget of a single L3 cache miss (~100ns on
modern hardware).

## Backpressure

When all 8190 slots are occupied, `claim_write_slot()` returns
`BusError::BusFull`. The Shield responds to the client immediately with
HTTP 503 Service Unavailable. This is the correct behaviour: the alternative
(queuing in memory) can cause OOM crashes under sustained overload.

The number of slots (8190) is tuned so that under the expected peak load
(100K concurrent requests), the average occupancy is below 10%. The ratio can
be adjusted by changing `SEGMENT_SIZE` and `SLOT_SIZE` in
`ag-core/src/bus/ring_buffer.rs`.

## Code Generation (Phase 3)

The Anti-DSL compiler (`ag-core/src/dsl/`) reads `.ag` schema files and
generates:

- Rust: validation structs and schema validators for the Shield
- Go: handler stubs, model types, and sqlc query wrappers for the Brain
- TypeScript: types and a typed HTTP client for frontend consumers
- OpenAPI 3.1: machine-readable API documentation

This means the `.ag` file is the single source of truth for the API contract.
Type drift between frontend and backend becomes a compile error, not a runtime
bug.
