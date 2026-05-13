/// The root node of a parsed `.ag` schema file.
#[derive(Debug, Clone, PartialEq)]
pub struct SchemaFile {
    pub models: Vec<ModelDef>,
    pub requests: Vec<RequestDef>,
    pub endpoints: Vec<EndpointDef>,
    pub enums: Vec<EnumDef>,
}

/// A `model` declaration.
///
/// ```ag
/// model User {
///   id      UUID      @primary @auto
///   email   String    @unique @max(255)
///   name    String    @max(100)
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ModelDef {
    pub name: String,
    pub fields: Vec<FieldDef>,
}

/// A `request` declaration (separate from model — used as request body type).
///
/// ```ag
/// request CreateUserRequest {
///   email String @email
///   name  String @min(2) @max(100)
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct RequestDef {
    pub name: String,
    pub fields: Vec<FieldDef>,
}

/// An `enum` declaration.
///
/// ```ag
/// enum UserRole {
///   USER
///   ADMIN
///   SUPER_ADMIN
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct EnumDef {
    pub name: String,
    pub variants: Vec<String>,
}

/// A single field inside a model or request.
#[derive(Debug, Clone, PartialEq)]
pub struct FieldDef {
    pub name: String,
    pub ty: FieldType,
    pub directives: Vec<Directive>,
    pub optional: bool,
}

/// Built-in scalar types supported by the Anti-Gravital DSL.
#[derive(Debug, Clone, PartialEq)]
pub enum FieldType {
    /// RFC 4122 UUID
    Uuid,
    /// RFC 5321 email address (alias for String with @email)
    Email,
    /// Unicode string
    String,
    /// 64-bit signed integer
    Int,
    /// 64-bit IEEE 754 float
    Float,
    /// Boolean
    Bool,
    /// RFC 3339 timestamp
    Timestamp,
    /// JSON blob
    Json,
    /// Reference to another model or enum by name
    Model(String),
    /// Array of another type
    Array(Box<FieldType>),
}

/// Field-level decorator that affects validation, indexing, or code generation.
#[derive(Debug, Clone, PartialEq)]
pub enum Directive {
    /// Marks the field as the primary key.
    Primary,
    /// Field is populated automatically (e.g., `created_at`, UUID).
    Auto,
    /// Enforces uniqueness at the database and schema level.
    Unique,
    /// Creates a database index on this field.
    Index,
    /// Field can be null/None.
    Nullable,
    /// Field value is encrypted at rest.
    Encrypted,
    /// Validates as an email address.
    Email,
    /// Maximum length (from `@max(n)`).
    MaxLen(u64),
    /// Minimum length (from `@min(n)`).
    MinLen(u64),
    /// Default value (from `@default(val)`).
    Default(String),
    /// Format hint (from `@format(val)`).
    Format(String),
    /// Arbitrary named directive with optional argument (fallback).
    Named { name: String, arg: Option<String> },
}

/// An `endpoint` declaration.
///
/// ```ag
/// endpoint CreateUser {
///   method   POST
///   path     /users
///   auth     required
///   body     CreateUserRequest
///   response User
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct EndpointDef {
    pub name: String,
    pub method: HttpMethod,
    pub path: String,
    pub auth: AuthRequirement,
    pub policy: Option<String>,
    pub body: Option<String>,
    pub response_type: String,
    pub errors: Vec<String>,
}

/// HTTP verbs recognised by the DSL.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
}

impl std::fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HttpMethod::Get => write!(f, "GET"),
            HttpMethod::Post => write!(f, "POST"),
            HttpMethod::Put => write!(f, "PUT"),
            HttpMethod::Patch => write!(f, "PATCH"),
            HttpMethod::Delete => write!(f, "DELETE"),
        }
    }
}

/// Authentication requirement for an endpoint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthRequirement {
    /// Authentication is required.
    Required,
    /// Authentication is optional.
    Optional,
    /// No authentication needed.
    None,
}

impl Default for AuthRequirement {
    fn default() -> Self {
        AuthRequirement::None
    }
}
