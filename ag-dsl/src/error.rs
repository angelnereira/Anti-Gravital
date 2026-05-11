use thiserror::Error;

#[derive(Debug, Error)]
pub enum DslError {
    #[error("unexpected character '{ch}' at {line}:{col}")]
    UnexpectedChar { ch: char, line: usize, col: usize },

    #[error("unterminated string literal at line {line}")]
    UnterminatedString { line: usize },

    #[error("parse error at line {line}: {message}")]
    Parse { line: usize, message: String },

    #[error("semantic error: {0}")]
    Semantic(String),

    #[error("codegen error: {0}")]
    Codegen(String),
}
