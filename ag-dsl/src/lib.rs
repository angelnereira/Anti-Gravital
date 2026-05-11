pub mod ast;
pub mod codegen;
pub mod error;
pub mod lexer;
pub mod parser;
pub mod semantic;

use crate::error::DslError;
use crate::lexer::Lexer;
use crate::parser::Parser;

/// Compiles a `.ag` schema source string into an AST, runs semantic checks,
/// and returns the validated [`ast::SchemaFile`].
pub fn compile(src: &str) -> Result<ast::SchemaFile, DslError> {
    let tokens = Lexer::new(src).tokenize()?;
    let schema = Parser::new(tokens).parse()?;
    semantic::check(&schema)?;
    Ok(schema)
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXAMPLE: &str = r#"
model User {
  id: UUID @primary
  email: Email @unique
  name: String
}

endpoint GET /users/{id} -> User
  auth: jwt
  validate: strict
"#;

    #[test]
    fn compile_example_succeeds() {
        let schema = compile(EXAMPLE).unwrap();
        assert_eq!(schema.models.len(), 1);
        assert_eq!(schema.endpoints.len(), 1);
    }
}
