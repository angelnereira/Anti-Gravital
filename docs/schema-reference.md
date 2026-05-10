# Schema Reference

The Anti-DSL (`.ag` files) is the single source of truth for an Anti-Gravital
application. The `ag generate` command reads the schema and produces type-safe
code in Rust, Go, TypeScript, and OpenAPI.

*The parser is implemented in Phase 3. This document defines the intended
language specification.*

## File Header

Every schema file begins with a version and namespace declaration:

```
@version 1.0
@namespace api.users
```

The namespace is used as a Go package prefix and a Rust module prefix for all
generated code.

## Primitive Types

| Type        | Description                                      | Go type        | Rust type        |
|-------------|--------------------------------------------------|----------------|------------------|
| `String`    | UTF-8 string                                     | `string`       | `String`         |
| `Int`       | 64-bit signed integer                            | `int64`        | `i64`            |
| `Float`     | 64-bit IEEE 754 float                            | `float64`      | `f64`            |
| `Bool`      | Boolean                                          | `bool`         | `bool`           |
| `UUID`      | RFC 4122 UUID (stored as text in most databases) | `uuid.UUID`    | `uuid::Uuid`     |
| `Timestamp` | RFC 3339 timestamp                               | `time.Time`    | `chrono::DateTime<Utc>` |
| `Date`      | Calendar date (no time component)                | `time.Time`    | `chrono::NaiveDate` |
| `Money`     | Decimal with 2 fractional digits, no float       | `decimal.Decimal` | `rust_decimal::Decimal` |
| `Email`     | String validated as an email address             | `string`       | `String`         |
| `URL`       | String validated as an absolute URL              | `string`       | `String`         |
| `Phone`     | E.164 phone number string                        | `string`       | `String`         |

## Validation Modifiers

Modifiers are applied to fields using the `@` prefix.

### Size and Range

| Modifier     | Applies To         | Description                           |
|--------------|--------------------|---------------------------------------|
| `@min(n)`    | String, Int, Float | Minimum length (String) or value      |
| `@max(n)`    | String, Int, Float | Maximum length (String) or value      |
| `@positive`  | Int, Float, Money  | Value must be greater than zero       |

### Format and Pattern

| Modifier           | Applies To | Description                          |
|--------------------|------------|--------------------------------------|
| `@email`           | String     | Shorthand for `@format(email)`       |
| `@format(f)`       | String     | Named format: email, url, uuid, etc. |
| `@regex(pattern)`  | String     | ECMAScript regex pattern             |

### Database Behavior

| Modifier       | Description                                             |
|----------------|---------------------------------------------------------|
| `@primary`     | Marks the field as the primary key                      |
| `@auto`        | Server-generated (UUID v7, current timestamp, serial)   |
| `@unique`      | Unique constraint in the database                       |
| `@index`       | Database index (non-unique)                             |
| `@nullable`    | Field may be null/nil in code and NULL in the database  |
| `@encrypted`   | Value is encrypted at rest using the application key    |
| `@default(v)`  | Default value used when the field is omitted in writes  |

### Relationships

```
model Order {
    id      UUID   @primary @auto
    user    User   @relation(field: userId)
    userId  UUID
    items   Item[] @relation(onDelete: CASCADE)
}
```

## Model Definitions

```
model User {
    id       UUID      @primary @auto
    email    String    @unique @max(255) @format(email)
    name     String    @max(100)
    role     UserRole  @default(USER)
    created  Timestamp @auto
}
```

## Enum Definitions

```
enum UserRole {
    USER
    ADMIN
    SUPER_ADMIN
}
```

## Request Types

Request types define the shape of request bodies. They are separate from
models because they typically omit server-generated fields like `id` and
`created`.

```
request CreateUserRequest {
    email  String  @email
    name   String  @min(2) @max(100)
}
```

## Endpoint Definitions

```
endpoint CreateUser {
    method    POST
    path      /users
    auth      required
    body      CreateUserRequest
    response  User
    errors    [EmailTaken, ValidationError]
}
```

| Field       | Description                                              |
|-------------|----------------------------------------------------------|
| `method`    | HTTP method: GET, POST, PUT, PATCH, DELETE               |
| `path`      | URL path, supports `:param` segments                     |
| `auth`      | `required` or `optional`; omit for public endpoints      |
| `body`      | Request body type (required for POST/PUT/PATCH)          |
| `query`     | Query string parameters (see below)                      |
| `response`  | Response body type                                       |
| `errors`    | List of named error types this endpoint can return       |

### Query Parameters

```
endpoint ListUsers {
    method    GET
    path      /users
    auth      required
    query     { page: Int @default(1), limit: Int @default(20) }
    response  PaginatedResponse[User]
}
```

### Authorization Policies

```
endpoint GetOrder {
    method    GET
    path      /orders/:id
    auth      required
    policy    "user.id == :userId OR user.role == ADMIN"
    response  Order
}
```

The policy expression is validated at schema-lint time to ensure it references
fields that exist on the authenticated user's claims.

## Event Definitions

```
event OrderCreated {
    order   Order
    userId  UUID
    at      Timestamp
}
```

Defining an event generates a typed producer function in Go and a subscriber
type. Events are published through AG-Realtime (NATS).

## Auth Configuration

```
auth {
    providers [webauthn, jwt, oauth2]
    jwt {
        algorithm  Ed25519
        expiry     15m
        refresh    7d
    }
}
```

## Storage Configuration

```
storage {
    provider  s3
    bucket    app-uploads
    max_size  50MB
    allowed   [image/jpeg, image/png, application/pdf]
}
```

## Generated Artifacts

Running `ag generate` produces the following from a schema file:

| File                       | Description                                    |
|----------------------------|------------------------------------------------|
| `src/rust/models.rs`       | Rust structs for Shield validation             |
| `src/rust/validators.rs`   | Schema validation logic for the Shield         |
| `src/go/models.go`         | Go structs matching the Rust models            |
| `src/go/handlers_stubs.go` | Unimplemented handler stubs to fill in         |
| `src/go/queries.sql.go`    | sqlc-generated type-safe query functions       |
| `src/ts/types.ts`          | TypeScript type declarations for the frontend  |
| `src/ts/client.ts`         | Typed HTTP client generated from endpoints     |
| `openapi.yaml`             | OpenAPI 3.1 specification                      |
