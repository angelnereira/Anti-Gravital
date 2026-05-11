use crate::ast::*;
use crate::error::DslError;
use crate::lexer::{Lexeme, Token};

pub struct Parser {
    tokens: Vec<Lexeme>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Lexeme>) -> Self {
        Self { tokens, pos: 0 }
    }

    pub fn parse(mut self) -> Result<SchemaFile, DslError> {
        let mut models = Vec::new();
        let mut endpoints = Vec::new();

        loop {
            self.skip_newlines();
            match self.current_token() {
                Token::Eof => break,
                Token::Model => models.push(self.parse_model()?),
                Token::Endpoint => endpoints.push(self.parse_endpoint()?),
                _ => {
                    return Err(DslError::Parse {
                        line: self.current_line(),
                        message: format!(
                            "expected 'model' or 'endpoint', found {:?}",
                            self.current_token()
                        ),
                    })
                }
            }
        }

        Ok(SchemaFile { models, endpoints })
    }

    fn parse_model(&mut self) -> Result<ModelDef, DslError> {
        self.expect(Token::Model)?;
        let name = self.expect_ident()?;
        self.skip_newlines();
        self.expect(Token::LBrace)?;

        let mut fields = Vec::new();
        loop {
            self.skip_newlines();
            if matches!(self.current_token(), Token::RBrace | Token::Eof) {
                break;
            }
            fields.push(self.parse_field()?);
        }
        self.expect(Token::RBrace)?;

        Ok(ModelDef { name, fields })
    }

    fn parse_field(&mut self) -> Result<FieldDef, DslError> {
        let name = self.expect_ident()?;

        let optional = if matches!(self.current_token(), Token::Question) {
            self.advance();
            true
        } else {
            false
        };

        self.expect(Token::Colon)?;
        let ty = self.parse_field_type()?;

        let mut directives = Vec::new();
        while matches!(self.current_token(), Token::At) {
            directives.push(self.parse_directive()?);
        }

        Ok(FieldDef { name, ty, directives, optional })
    }

    fn parse_field_type(&mut self) -> Result<FieldType, DslError> {
        let name = self.expect_ident()?;
        let ty = match name.as_str() {
            "UUID" => FieldType::Uuid,
            "Email" => FieldType::Email,
            "String" => FieldType::String,
            "Int" => FieldType::Int,
            "Float" => FieldType::Float,
            "Bool" => FieldType::Bool,
            "Timestamp" => FieldType::Timestamp,
            "Json" => FieldType::Json,
            other => FieldType::Model(other.to_owned()),
        };
        Ok(ty)
    }

    fn parse_directive(&mut self) -> Result<Directive, DslError> {
        self.expect(Token::At)?;
        let name = self.expect_ident()?;
        let directive = match name.as_str() {
            "primary" => Directive::Primary,
            "unique" => Directive::Unique,
            "auto" => Directive::Auto,
            other => {
                let arg = if matches!(self.current_token(), Token::StringLit(_)) {
                    Some(self.expect_string()?)
                } else {
                    None
                };
                Directive::Named { name: other.to_owned(), arg }
            }
        };
        Ok(directive)
    }

    fn parse_endpoint(&mut self) -> Result<EndpointDef, DslError> {
        self.expect(Token::Endpoint)?;

        let method = match self.current_token() {
            Token::Get => { self.advance(); HttpMethod::Get }
            Token::Post => { self.advance(); HttpMethod::Post }
            Token::Put => { self.advance(); HttpMethod::Put }
            Token::Patch => { self.advance(); HttpMethod::Patch }
            Token::Delete => { self.advance(); HttpMethod::Delete }
            other => return Err(DslError::Parse {
                line: self.current_line(),
                message: format!("expected HTTP method, found {other:?}"),
            }),
        };

        let path = self.expect_path()?;

        self.expect(Token::Arrow)?;
        let response_type = self.expect_ident()?;

        let mut options = Vec::new();
        while matches!(self.current_token(), Token::Newline) {
            self.advance();
            self.skip_newlines();
            if !matches!(self.current_token(), Token::Ident(_)) {
                break;
            }
            let key = self.expect_ident()?;
            self.expect(Token::Colon)?;
            let value = self.expect_ident()?;
            let opt = match key.as_str() {
                "auth" => EndpointOption::Auth(value),
                "cache" => EndpointOption::Cache(value),
                "validate" => EndpointOption::Validate(value),
                other => EndpointOption::Named { key: other.to_owned(), value },
            };
            options.push(opt);
        }

        Ok(EndpointDef { method, path, response_type, options })
    }

    fn current_token(&self) -> &Token {
        &self.tokens[self.pos].token
    }

    fn current_line(&self) -> usize {
        self.tokens[self.pos].span.line
    }

    fn advance(&mut self) {
        if self.pos + 1 < self.tokens.len() {
            self.pos += 1;
        }
    }

    fn skip_newlines(&mut self) {
        while matches!(self.current_token(), Token::Newline) {
            self.advance();
        }
    }

    fn expect(&mut self, expected: Token) -> Result<(), DslError> {
        if std::mem::discriminant(self.current_token()) == std::mem::discriminant(&expected) {
            self.advance();
            Ok(())
        } else {
            Err(DslError::Parse {
                line: self.current_line(),
                message: format!("expected {expected:?}, found {:?}", self.current_token()),
            })
        }
    }

    fn expect_ident(&mut self) -> Result<String, DslError> {
        match self.current_token().clone() {
            Token::Ident(s) => {
                self.advance();
                Ok(s)
            }
            other => Err(DslError::Parse {
                line: self.current_line(),
                message: format!("expected identifier, found {other:?}"),
            }),
        }
    }

    fn expect_path(&mut self) -> Result<String, DslError> {
        match self.current_token().clone() {
            Token::Path(s) => {
                self.advance();
                Ok(s)
            }
            other => Err(DslError::Parse {
                line: self.current_line(),
                message: format!("expected path, found {other:?}"),
            }),
        }
    }

    fn expect_string(&mut self) -> Result<String, DslError> {
        match self.current_token().clone() {
            Token::StringLit(s) => {
                self.advance();
                Ok(s)
            }
            other => Err(DslError::Parse {
                line: self.current_line(),
                message: format!("expected string literal, found {other:?}"),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;

    fn parse(src: &str) -> SchemaFile {
        let tokens = Lexer::new(src).tokenize().unwrap();
        Parser::new(tokens).parse().unwrap()
    }

    #[test]
    fn parses_empty_model() {
        let schema = parse("model Empty {}");
        assert_eq!(schema.models.len(), 1);
        assert_eq!(schema.models[0].name, "Empty");
        assert!(schema.models[0].fields.is_empty());
    }

    #[test]
    fn parses_model_with_fields() {
        let schema = parse(
            "model User {\n  id: UUID @primary\n  email: Email @unique\n  name: String\n}",
        );
        let model = &schema.models[0];
        assert_eq!(model.fields.len(), 3);
        assert_eq!(model.fields[0].name, "id");
        assert_eq!(model.fields[0].ty, FieldType::Uuid);
        assert!(model.fields[0].directives.contains(&Directive::Primary));
    }

    #[test]
    fn parses_endpoint() {
        let schema = parse("endpoint GET /users/{id} -> User");
        assert_eq!(schema.endpoints.len(), 1);
        let ep = &schema.endpoints[0];
        assert_eq!(ep.method, HttpMethod::Get);
        assert_eq!(ep.path, "/users/{id}");
        assert_eq!(ep.response_type, "User");
    }
}
