/// The root node of a parsed `.ag` schema file.
#[derive(Debug, Clone, PartialEq)]
pub struct SchemaFile {
    pub models: Vec<ModelDef>,
    pub endpoints: Vec<EndpointDef>,
}

/// A `model` declaration.
///
/// ```ag
/// model User {
///   id: UUID @primary
///   email: Email @unique
///   name: String
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ModelDef {
    pub name: String,
    pub fields: Vec<FieldDef>,
}

/// A single field inside a model.
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
    /// RFC 5321 email address
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
    /// Reference to another model by name
    Model(String),
    /// Array of another type
    Array(Box<FieldType>),
}

/// Field-level decorator that affects validation, indexing, or code generation.
#[derive(Debug, Clone, PartialEq)]
pub enum Directive {
    /// Marks the field as the primary key.
    Primary,
    /// Enforces uniqueness at the database and schema level.
    Unique,
    /// Field is populated automatically (e.g., `created_at`, `updated_at`).
    Auto,
    /// Arbitrary named directive with optional string argument.
    Named { name: String, arg: Option<String> },
}

/// An `endpoint` declaration.
///
/// ```ag
/// endpoint GET /users/{id} -> User
///   auth: jwt
///   cache: 5m
///   validate: strict
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct EndpointDef {
    pub method: HttpMethod,
    pub path: String,
    pub response_type: String,
    pub options: Vec<EndpointOption>,
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

/// Endpoint-level options that affect middleware selection.
#[derive(Debug, Clone, PartialEq)]
pub enum EndpointOption {
    /// Authentication strategy (`jwt`, `api-key`, `none`).
    Auth(String),
    /// Cache TTL (e.g., `5m`, `30s`).
    Cache(String),
    /// Validation mode (`strict`, `loose`).
    Validate(String),
    /// Arbitrary named option.
    Named { key: String, value: String },
}
