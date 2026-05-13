use crate::error::DslError;

/// All tokens produced by the v3.0 `.ag` lexer.
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Top-level keywords
    Model,
    Request,
    Endpoint,
    Enum,
    Event,

    // Endpoint block field keywords
    Method,
    Path,
    Auth,
    Body,
    Response,
    Errors,
    Policy,

    // Auth value keywords
    Required,
    Optional,

    // HTTP method keywords (only when standalone)
    Get,
    Post,
    Put,
    Patch,
    Delete,

    // Punctuation
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    LParen,
    RParen,
    Comma,
    At,
    Question,

    // Literals
    Ident(String),
    PathLit(String),
    StringLit(String),
    Number(String),

    // Whitespace / structure
    Newline,
    Eof,
}

#[derive(Debug, Clone)]
pub struct Span {
    pub line: usize,
    pub col: usize,
}

#[derive(Debug, Clone)]
pub struct Lexeme {
    pub token: Token,
    pub span: Span,
}

pub struct Lexer<'src> {
    src: &'src str,
    pos: usize,
    line: usize,
    col: usize,
}

impl<'src> Lexer<'src> {
    pub fn new(src: &'src str) -> Self {
        Self { src, pos: 0, line: 1, col: 1 }
    }

    pub fn tokenize(mut self) -> Result<Vec<Lexeme>, DslError> {
        let mut tokens = Vec::new();

        loop {
            self.skip_whitespace_inline();

            if self.pos >= self.src.len() {
                tokens.push(Lexeme { token: Token::Eof, span: self.span() });
                break;
            }

            let ch = self.current();

            if ch == '#' {
                self.skip_line_comment();
                continue;
            }

            if ch == '\n' {
                let span = self.span();
                self.advance();
                tokens.push(Lexeme { token: Token::Newline, span });
                continue;
            }

            if ch == '\r' {
                self.advance();
                continue;
            }

            if ch == '{' {
                let span = self.span();
                self.advance();
                tokens.push(Lexeme { token: Token::LBrace, span });
                continue;
            }

            if ch == '}' {
                let span = self.span();
                self.advance();
                tokens.push(Lexeme { token: Token::RBrace, span });
                continue;
            }

            if ch == '[' {
                let span = self.span();
                self.advance();
                tokens.push(Lexeme { token: Token::LBracket, span });
                continue;
            }

            if ch == ']' {
                let span = self.span();
                self.advance();
                tokens.push(Lexeme { token: Token::RBracket, span });
                continue;
            }

            if ch == '(' {
                let span = self.span();
                self.advance();
                tokens.push(Lexeme { token: Token::LParen, span });
                continue;
            }

            if ch == ')' {
                let span = self.span();
                self.advance();
                tokens.push(Lexeme { token: Token::RParen, span });
                continue;
            }

            if ch == ',' {
                let span = self.span();
                self.advance();
                tokens.push(Lexeme { token: Token::Comma, span });
                continue;
            }

            if ch == '@' {
                let span = self.span();
                self.advance();
                tokens.push(Lexeme { token: Token::At, span });
                continue;
            }

            if ch == '?' {
                let span = self.span();
                self.advance();
                tokens.push(Lexeme { token: Token::Question, span });
                continue;
            }

            if ch == '/' {
                let span = self.span();
                let path = self.read_path();
                tokens.push(Lexeme { token: Token::PathLit(path), span });
                continue;
            }

            if ch == '"' {
                let span = self.span();
                let s = self.read_string_literal()?;
                tokens.push(Lexeme { token: Token::StringLit(s), span });
                continue;
            }

            if ch.is_ascii_digit() {
                let span = self.span();
                let n = self.read_number();
                tokens.push(Lexeme { token: Token::Number(n), span });
                continue;
            }

            if ch.is_ascii_alphabetic() || ch == '_' {
                let span = self.span();
                let word = self.read_ident();
                let tok = keyword_or_ident(word);
                tokens.push(Lexeme { token: tok, span });
                continue;
            }

            return Err(DslError::UnexpectedChar { ch, line: self.line, col: self.col });
        }

        Ok(tokens)
    }

    fn current(&self) -> char {
        self.src[self.pos..].chars().next().unwrap_or('\0')
    }

    fn advance(&mut self) {
        if let Some(ch) = self.src[self.pos..].chars().next() {
            self.pos += ch.len_utf8();
            if ch == '\n' {
                self.line += 1;
                self.col = 1;
            } else {
                self.col += 1;
            }
        }
    }

    fn span(&self) -> Span {
        Span { line: self.line, col: self.col }
    }

    fn skip_whitespace_inline(&mut self) {
        while self.pos < self.src.len() {
            let ch = self.current();
            if ch == ' ' || ch == '\t' {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn skip_line_comment(&mut self) {
        while self.pos < self.src.len() && self.current() != '\n' {
            self.advance();
        }
    }

    fn read_ident(&mut self) -> String {
        let start = self.pos;
        while self.pos < self.src.len() {
            let ch = self.current();
            if ch.is_ascii_alphanumeric() || ch == '_' {
                self.advance();
            } else {
                break;
            }
        }
        self.src[start..self.pos].to_owned()
    }

    fn read_number(&mut self) -> String {
        let start = self.pos;
        while self.pos < self.src.len() {
            let ch = self.current();
            if ch.is_ascii_digit() || ch == '.' {
                self.advance();
            } else {
                break;
            }
        }
        self.src[start..self.pos].to_owned()
    }

    fn read_path(&mut self) -> String {
        let start = self.pos;
        while self.pos < self.src.len() {
            let ch = self.current();
            // Paths end at whitespace or newlines
            if ch == ' ' || ch == '\t' || ch == '\n' || ch == '\r' {
                break;
            }
            self.advance();
        }
        self.src[start..self.pos].to_owned()
    }

    fn read_string_literal(&mut self) -> Result<String, DslError> {
        self.advance(); // consume opening '"'
        let start = self.pos;
        while self.pos < self.src.len() && self.current() != '"' {
            if self.current() == '\n' {
                return Err(DslError::UnterminatedString { line: self.line });
            }
            self.advance();
        }
        if self.pos >= self.src.len() {
            return Err(DslError::UnterminatedString { line: self.line });
        }
        let s = self.src[start..self.pos].to_owned();
        self.advance(); // consume closing '"'
        Ok(s)
    }
}

fn keyword_or_ident(word: String) -> Token {
    match word.as_str() {
        // Top-level keywords
        "model" => Token::Model,
        "request" => Token::Request,
        "endpoint" => Token::Endpoint,
        "enum" => Token::Enum,
        "event" => Token::Event,
        // Endpoint block field keywords
        "method" => Token::Method,
        "path" => Token::Path,
        "auth" => Token::Auth,
        "body" => Token::Body,
        "response" => Token::Response,
        "errors" => Token::Errors,
        "policy" => Token::Policy,
        // Auth value keywords
        "required" => Token::Required,
        "optional" => Token::Optional,
        // HTTP methods (uppercase)
        "GET" => Token::Get,
        "POST" => Token::Post,
        "PUT" => Token::Put,
        "PATCH" => Token::Patch,
        "DELETE" => Token::Delete,
        _ => Token::Ident(word),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokenize(src: &str) -> Vec<Token> {
        Lexer::new(src).tokenize().unwrap().into_iter().map(|l| l.token).collect()
    }

    #[test]
    fn tokenizes_model_keyword() {
        let tokens = tokenize("model");
        assert!(matches!(tokens[0], Token::Model));
    }

    #[test]
    fn tokenizes_request_keyword() {
        let tokens = tokenize("request");
        assert!(matches!(tokens[0], Token::Request));
    }

    #[test]
    fn tokenizes_enum_keyword() {
        let tokens = tokenize("enum");
        assert!(matches!(tokens[0], Token::Enum));
    }

    #[test]
    fn tokenizes_http_method() {
        let tokens = tokenize("GET");
        assert!(matches!(tokens[0], Token::Get));
    }

    #[test]
    fn tokenizes_path_literal() {
        let tokens = tokenize("/users/{id}");
        assert!(matches!(&tokens[0], Token::PathLit(p) if p == "/users/{id}"));
    }

    #[test]
    fn tokenizes_at_directive() {
        let tokens = tokenize("@primary");
        assert!(matches!(tokens[0], Token::At));
        assert!(matches!(&tokens[1], Token::Ident(s) if s == "primary"));
    }

    #[test]
    fn tokenizes_directive_with_arg() {
        let tokens = tokenize("@max(255)");
        assert!(matches!(tokens[0], Token::At));
        assert!(matches!(&tokens[1], Token::Ident(s) if s == "max"));
        assert!(matches!(tokens[2], Token::LParen));
        assert!(matches!(&tokens[3], Token::Number(n) if n == "255"));
        assert!(matches!(tokens[4], Token::RParen));
    }

    #[test]
    fn tokenizes_directive_with_ident_arg() {
        let tokens = tokenize("@default(USER)");
        assert!(matches!(tokens[0], Token::At));
        assert!(matches!(&tokens[1], Token::Ident(s) if s == "default"));
        assert!(matches!(tokens[2], Token::LParen));
        assert!(matches!(&tokens[3], Token::Ident(s) if s == "USER"));
        assert!(matches!(tokens[4], Token::RParen));
    }

    #[test]
    fn tokenizes_brackets_for_errors_list() {
        let tokens = tokenize("[EmailTaken, ValidationError]");
        assert!(matches!(tokens[0], Token::LBracket));
        assert!(matches!(&tokens[1], Token::Ident(s) if s == "EmailTaken"));
        assert!(matches!(tokens[2], Token::Comma));
        assert!(matches!(&tokens[3], Token::Ident(s) if s == "ValidationError"));
        assert!(matches!(tokens[4], Token::RBracket));
    }

    #[test]
    fn skips_line_comments() {
        let tokens = tokenize("# comment\nmodel");
        // tokens: Newline, Model, Eof
        let non_eof: Vec<_> = tokens.iter().filter(|t| !matches!(t, Token::Eof)).collect();
        assert!(non_eof.iter().any(|t| matches!(t, Token::Model)));
    }

    #[test]
    fn tokenizes_string_literal() {
        let tokens = tokenize("\"user.role != BANNED\"");
        assert!(matches!(&tokens[0], Token::StringLit(s) if s == "user.role != BANNED"));
    }

    #[test]
    fn field_without_colon() {
        // Ensure no colon token is needed between field name and type
        let tokens = tokenize("id UUID");
        assert!(matches!(&tokens[0], Token::Ident(s) if s == "id"));
        assert!(matches!(&tokens[1], Token::Ident(s) if s == "UUID"));
    }
}
