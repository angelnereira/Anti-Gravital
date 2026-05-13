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
enum UserRole {
  USER
  ADMIN
}

model User {
  id    UUID      @primary @auto
  email String    @unique @max(255)
  name  String    @max(100)
  role  UserRole  @default(USER)
}

request CreateUserRequest {
  email String @email
  name  String @min(2) @max(100)
}

endpoint GetUser {
  method   GET
  path     /users/{id}
  auth     required
  response User
}

endpoint CreateUser {
  method   POST
  path     /users
  auth     required
  body     CreateUserRequest
  response User
}
"#;

    #[test]
    fn compile_example_succeeds() {
        let schema = compile(EXAMPLE).unwrap();
        assert_eq!(schema.models.len(), 1);
        assert_eq!(schema.enums.len(), 1);
        assert_eq!(schema.requests.len(), 1);
        assert_eq!(schema.endpoints.len(), 2);
    }

    #[test]
    fn compile_generates_all_files() {
        let schema = compile(EXAMPLE).unwrap();
        let files = codegen::generate_all(&schema).unwrap();
        assert!(files.len() >= 4);
        let paths: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();
        assert!(paths.contains(&"src/models.rs"), "missing src/models.rs");
        assert!(paths.contains(&"src/models.ts"), "missing src/models.ts");
        assert!(paths.contains(&"openapi.yaml"), "missing openapi.yaml");
    }
}
