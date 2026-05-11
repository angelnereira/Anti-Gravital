pub mod openapi;
pub mod rust;
pub mod typescript;

use crate::ast::SchemaFile;
use crate::error::DslError;

/// Output produced by a code generator.
#[derive(Debug, Clone)]
pub struct GeneratedFile {
    /// Relative path of the file to write (e.g., `src/models.rs`).
    pub path: String,
    /// The generated source text.
    pub content: String,
}

/// Runs all code generators and returns the set of files to write.
pub fn generate_all(schema: &SchemaFile) -> Result<Vec<GeneratedFile>, DslError> {
    let mut files = Vec::new();
    files.extend(rust::generate(schema)?);
    files.extend(typescript::generate(schema)?);
    files.push(openapi::generate(schema)?);
    Ok(files)
}
