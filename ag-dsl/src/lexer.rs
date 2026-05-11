use crate::error::DslError;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Keywords
    Model,
    Endpoint,

    // HTTP methods
    Get,
    Post,
    Put,
    Patch,
    Delete,

    // Punctuation
    LBrace,
    RBrace,
    Colon,
    At,
    Arrow,
    Question,

    // Literals
    Ident(String),
    Path(String),
    StringLit(String),

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

            if ch == ':' {
                let span = self.span();
                self.advance();
                tokens.push(Lexeme { token: Token::Colon, span });
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

            if ch == '-' && self.peek(1) == Some('>') {
                let span = self.span();
                self.advance();
                self.advance();
                tokens.push(Lexeme { token: Token::Arrow, span });
                continue;
            }

            if ch == '/' {
                let span = self.span();
                let path = self.read_path();
                tokens.push(Lexeme { token: Token::Path(path), span });
                continue;
            }

            if ch == '"' {
                let span = self.span();
                let s = self.read_string_literal()?;
                tokens.push(Lexeme { token: Token::StringLit(s), span });
                continue;
            }

            if ch.is_ascii_alphabetic() || ch == '_' {
                let span = self.span();
                let word = self.read_ident();
                let tok = match word.as_str() {
                    "model" => Token::Model,
                    "endpoint" => Token::Endpoint,
                    "GET" => Token::Get,
                    "POST" => Token::Post,
                    "PUT" => Token::Put,
                    "PATCH" => Token::Patch,
                    "DELETE" => Token::Delete,
                    _ => Token::Ident(word),
                };
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

    fn peek(&self, offset: usize) -> Option<char> {
        self.src[self.pos..].chars().nth(offset)
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
            if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenizes_model_keyword() {
        let tokens = Lexer::new("model").tokenize().unwrap();
        assert!(matches!(tokens[0].token, Token::Model));
    }

    #[test]
    fn tokenizes_arrow() {
        let tokens = Lexer::new("->").tokenize().unwrap();
        assert!(matches!(tokens[0].token, Token::Arrow));
    }

    #[test]
    fn tokenizes_http_method() {
        let tokens = Lexer::new("GET").tokenize().unwrap();
        assert!(matches!(tokens[0].token, Token::Get));
    }

    #[test]
    fn tokenizes_path() {
        let tokens = Lexer::new("/users/{id}").tokenize().unwrap();
        assert!(matches!(&tokens[0].token, Token::Path(p) if p == "/users/{id}"));
    }

    #[test]
    fn skips_line_comments() {
        let tokens = Lexer::new("# comment\nmodel").tokenize().unwrap();
        assert!(matches!(tokens[1].token, Token::Model));
    }
}
