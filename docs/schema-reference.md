# Schema Reference

The Anti-DSL (`.ag` files) is the single source of truth for an Anti-Gravital application. One file generates Rust structs, TypeScript interfaces, OpenAPI 3.1, sqlx queries, SQL migrations, and handler stubs. Schema drift is eliminated by design.

## Primitive Types

| Type | Description | Rust type |
|---|---|---|
| `String` | UTF-8 string | `String` |
| `Int` | 64-bit signed integer | `i64` |
| `Float` | 64-bit IEEE 754 float | `f64` |
| `Bool` | Boolean | `bool` |
| `UUID` | RFC 4122 UUID | `uuid::Uuid` |
| `Timestamp` | RFC 3339 timestamp | `chrono::DateTime<Utc>` |
| `Date` | Calendar date (no time) | `chrono::NaiveDate` |
| `Money` | Decimal with 2 fractional digits | `rust_decimal::Decimal` |
| `Email` | String validated as an email address | `String` |
| `URL` | String validated as an absolute URL | `String` |
| `Phone` | E.164 phone number | `String` |
| `Json` | Arbitrary JSON blob | `serde_json::Value` |

## Directives

Directives are applied to fields using the `@` prefix.

### Database

| Directive | Description |
|---|---|
| `@primary` | Marks the field as the primary key |
| `@auto` | Server-generated (UUID v7, current timestamp, serial) |
| `@unique` | Unique constraint in the database |
| `@index` | Non-unique database index |
| `@nullable` | Field may be null in both code and database |
| `@encrypted` | Value is encrypted at rest |
| `@default(v)` | Default value when field is omitted in writes |

### Validation

| Directive | Applies to | Description |
|---|---|---|
| `@min(n)` | String, Int, Float | Minimum length (String) or value |
| `@max(n)` | String, Int, Float | Maximum length (String) or value |
| `@positive` | Int, Float, Money | Value must be greater than zero |
| `@email` | String | Validates as an RFC 5321 email address |
| `@format(f)` | String | Named format: `email`, `url`, `uuid` |
| `@regex(p)` | String | ECMAScript regex pattern |

## Model Definitions

```
model User {
  id      UUID      @primary @auto
  email   String    @unique @max(255)
  name    String    @max(100)
  role    UserRole  @default(USER)
  created Timestamp @auto
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

Request types define the shape of request bodies. They are separate from models because they omit server-generated fields like `id` and `created`.

```
request CreateUserRequest {
  email String @email
  name  String @min(2) @max(100)
}
```

## Endpoint Definitions

```
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

| Field | Description |
|---|---|
| `method` | HTTP method: `GET`, `POST`, `PUT`, `PATCH`, `DELETE` |
| `path` | URL path, supports `{param}` segments |
| `auth` | `required` or `optional`; omit for public endpoints |
| `policy` | Authorization expression evaluated against JWT claims |
| `body` | Request body type (required for POST/PUT/PATCH) |
| `query` | Query string parameters |
| `response` | Response body type |
| `errors` | Named error types this endpoint can return |

### Query Parameters

```
endpoint ListUsers {
  method   GET
  path     /users
  auth     required
  query    { page: Int @default(1), limit: Int @default(20) }
  response User
}
```

### Authorization Policies

Policy expressions are validated at `ag schema lint` time to ensure they reference valid claim fields.

```
endpoint GetOrder {
  method   GET
  path     /orders/{id}
  auth     required
  policy   "user.id == :userId OR user.role == ADMIN"
  response Order
}
```

## Relationships

```
model Order {
  id     UUID   @primary @auto
  user   User   @relation(field: userId)
  userId UUID
  items  Item[] @relation(onDelete: CASCADE)
}
```

## Event Definitions

Events generate a typed Rust producer function and a subscriber type. Events are published through AG-Realtime (NATS).

```
event OrderCreated {
  order  Order
  userId UUID
  at     Timestamp
}
```

## Auth Configuration

```
auth {
  providers [webauthn, jwt, oauth2]
  jwt {
    algorithm Ed25519
    expiry    15m
    refresh   7d
  }
}
```

## Storage Configuration

```
storage {
  provider s3
  bucket   app-uploads
  max_size 50MB
  allowed  [image/jpeg, image/png, application/pdf]
}
```

## Generated Artifacts

`ag generate` produces the following from a schema file:

| File | Description |
|---|---|
| `src/models.rs` | Rust structs with serde derives |
| `src/validators.rs` | Shield validation logic |
| `src/handlers/stubs.rs` | Handler signatures — fill in the body |
| `src/db/queries.rs` | sqlx queries verified at compile time |
| `src/db/migrations/` | Versioned SQL migration files |
| `ts/types.ts` | TypeScript type declarations |
| `ts/client.ts` | Typed HTTP client generated from endpoints |
| `openapi.yaml` | OpenAPI 3.1 specification |

A change to `schema.ag` followed by `ag generate` updates all of the above atomically. There is no schema drift by construction.
