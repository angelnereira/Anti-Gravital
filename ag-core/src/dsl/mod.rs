/// Errors produced by the Anti-DSL compiler.
#[derive(Debug, thiserror::Error)]
pub enum DslError {
    #[error("lexer error at line {line}: {message}")]
    Lex { line: usize, message: String },
    #[error("parse error at line {line}: {message}")]
    Parse { line: usize, message: String },
    #[error("semantic error: {0}")]
    Semantic(String),
    #[error("code generation failed for target '{target}': {message}")]
    Codegen { target: String, message: String },
}

/// A parsed Anti-DSL schema file.
///
/// Populated by [`parse`] in Phase 3. In Phase 0 the struct exists to
/// establish the public API shape.
#[derive(Debug, Default)]
pub struct SchemaFile {
    pub version: String,
    pub namespace: String,
    pub models: Vec<ModelDef>,
    pub endpoints: Vec<EndpointDef>,
    pub requests: Vec<RequestDef>,
    pub enums: Vec<EnumDef>,
}

/// A model definition from the schema.
#[derive(Debug, Clone)]
pub struct ModelDef {
    pub name: String,
    pub fields: Vec<FieldDef>,
}

/// A field within a model or request type.
#[derive(Debug, Clone)]
pub struct FieldDef {
    pub name: String,
    pub type_name: String,
    pub modifiers: Vec<Modifier>,
}

/// A validation modifier applied to a field.
#[derive(Debug, Clone)]
pub enum Modifier {
    Primary,
    Auto,
    Unique,
    Nullable,
    Index,
    Encrypted,
    Min(i64),
    Max(i64),
    Default(String),
    Regex(String),
    Format(String),
    Relation { field: String, on_delete: Option<String> },
}

/// An endpoint definition from the schema.
#[derive(Debug, Clone)]
pub struct EndpointDef {
    pub name: String,
    pub method: String,
    pub path: String,
    pub auth: Option<String>,
    pub body: Option<String>,
    pub response: String,
    pub errors: Vec<String>,
}

/// A request body type definition.
#[derive(Debug, Clone)]
pub struct RequestDef {
    pub name: String,
    pub fields: Vec<FieldDef>,
}

/// An enum definition from the schema.
#[derive(Debug, Clone)]
pub struct EnumDef {
    pub name: String,
    pub variants: Vec<String>,
}

/// Parses an Anti-DSL schema source string into a [`SchemaFile`].
///
/// The full parser is implemented in Phase 3. This stub preserves the
/// public API and returns a descriptive error.
pub fn parse(_source: &str) -> Result<SchemaFile, DslError> {
    Err(DslError::Parse {
        line: 1,
        message: "Anti-DSL parser is scheduled for Phase 3. \
                  Use `ag generate` once the parser is implemented."
            .to_string(),
    })
}

/// Validates a parsed schema for semantic consistency.
///
/// Phase 3 implementation checks: circular references, undefined types,
/// policy field references, N+1 query warnings.
pub fn validate(_schema: &SchemaFile) -> Result<(), Vec<DslError>> {
    Ok(())
}
