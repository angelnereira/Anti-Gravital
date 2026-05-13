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
        let mut requests = Vec::new();
        let mut endpoints = Vec::new();
        let mut enums = Vec::new();

        loop {
            self.skip_newlines();
            match self.current_token().clone() {
                Token::Eof => break,
                Token::Model => models.push(self.parse_model()?),
                Token::Request => requests.push(self.parse_request()?),
                Token::Endpoint => endpoints.push(self.parse_endpoint()?),
                Token::Enum => enums.push(self.parse_enum()?),
                tok => {
                    return Err(DslError::Parse {
                        line: self.current_line(),
                        message: format!(
                            "expected 'model', 'request', 'endpoint', or 'enum', found {:?}",
                            tok
                        ),
                    })
                }
            }
        }

        Ok(SchemaFile { models, requests, endpoints, enums })
    }

    // ── model ──────────────────────────────────────────────────────────────

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

    // ── request ────────────────────────────────────────────────────────────

    fn parse_request(&mut self) -> Result<RequestDef, DslError> {
        self.expect(Token::Request)?;
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

        Ok(RequestDef { name, fields })
    }

    // ── enum ───────────────────────────────────────────────────────────────

    fn parse_enum(&mut self) -> Result<EnumDef, DslError> {
        self.expect(Token::Enum)?;
        let name = self.expect_ident()?;
        self.skip_newlines();
        self.expect(Token::LBrace)?;

        let mut variants = Vec::new();
        loop {
            self.skip_newlines();
            match self.current_token().clone() {
                Token::RBrace | Token::Eof => break,
                Token::Ident(v) => {
                    self.advance();
                    variants.push(v);
                }
                // HTTP-method tokens like GET, POST may appear as variant names
                Token::Get => { self.advance(); variants.push("GET".into()); }
                Token::Post => { self.advance(); variants.push("POST".into()); }
                Token::Put => { self.advance(); variants.push("PUT".into()); }
                Token::Patch => { self.advance(); variants.push("PATCH".into()); }
                Token::Delete => { self.advance(); variants.push("DELETE".into()); }
                tok => {
                    return Err(DslError::Parse {
                        line: self.current_line(),
                        message: format!("expected enum variant identifier, found {:?}", tok),
                    })
                }
            }
        }
        self.expect(Token::RBrace)?;

        Ok(EnumDef { name, variants })
    }

    // ── field ──────────────────────────────────────────────────────────────

    /// Parses `name Type[?] [@directive]* NEWLINE`
    ///
    /// No colon between name and type.
    fn parse_field(&mut self) -> Result<FieldDef, DslError> {
        // The field name can be a plain ident or a keyword used as a name
        // (e.g., someone naming a field "method" inside a model).
        let name = self.expect_any_ident()?;

        // Parse the type (mandatory)
        let (ty, optional) = self.parse_field_type()?;

        // Parse zero or more directives on the same logical line
        let mut directives = Vec::new();
        while matches!(self.current_token(), Token::At) {
            directives.push(self.parse_directive()?);
        }

        // Consume trailing newline(s) — but only on the same conceptual line.
        // Since newlines are insignificant inside blocks we just skip them.
        while matches!(self.current_token(), Token::Newline) {
            self.advance();
        }

        Ok(FieldDef { name, ty, directives, optional })
    }

    fn parse_field_type(&mut self) -> Result<(FieldType, bool), DslError> {
        let name = self.expect_any_ident()?;
        let mut ty = match name.as_str() {
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

        // Check for array suffix `[]`
        if matches!(self.current_token(), Token::LBracket) {
            self.advance();
            self.expect(Token::RBracket)?;
            ty = FieldType::Array(Box::new(ty));
        }

        // Check for optional suffix `?`
        let optional = if matches!(self.current_token(), Token::Question) {
            self.advance();
            true
        } else {
            false
        };

        Ok((ty, optional))
    }

    fn parse_directive(&mut self) -> Result<Directive, DslError> {
        self.expect(Token::At)?;
        let name = self.expect_any_ident()?;

        // Check whether there is an argument list `(arg)`
        let arg = if matches!(self.current_token(), Token::LParen) {
            self.advance(); // consume '('
            let arg_str = match self.current_token().clone() {
                Token::Number(n) => { self.advance(); n }
                Token::Ident(s) => { self.advance(); s }
                Token::StringLit(s) => { self.advance(); s }
                tok => {
                    return Err(DslError::Parse {
                        line: self.current_line(),
                        message: format!("expected directive argument, found {:?}", tok),
                    })
                }
            };
            self.expect(Token::RParen)?;
            Some(arg_str)
        } else {
            None
        };

        let directive = match (name.as_str(), arg.as_deref()) {
            ("primary", None) => Directive::Primary,
            ("auto", None) => Directive::Auto,
            ("unique", None) => Directive::Unique,
            ("index", None) => Directive::Index,
            ("nullable", None) => Directive::Nullable,
            ("encrypted", None) => Directive::Encrypted,
            ("email", None) => Directive::Email,
            ("max", Some(n)) => Directive::MaxLen(
                n.parse::<u64>().map_err(|_| DslError::Parse {
                    line: self.current_line(),
                    message: format!("@max argument must be a non-negative integer, got '{n}'"),
                })?
            ),
            ("min", Some(n)) => Directive::MinLen(
                n.parse::<u64>().map_err(|_| DslError::Parse {
                    line: self.current_line(),
                    message: format!("@min argument must be a non-negative integer, got '{n}'"),
                })?
            ),
            ("default", Some(val)) => Directive::Default(val.to_owned()),
            ("format", Some(val)) => Directive::Format(val.to_owned()),
            _ => Directive::Named { name, arg },
        };

        Ok(directive)
    }

    // ── endpoint ───────────────────────────────────────────────────────────

    fn parse_endpoint(&mut self) -> Result<EndpointDef, DslError> {
        self.expect(Token::Endpoint)?;
        let name = self.expect_ident()?;
        self.skip_newlines();
        self.expect(Token::LBrace)?;

        let mut method: Option<HttpMethod> = None;
        let mut path: Option<String> = None;
        let mut auth = AuthRequirement::None;
        let mut policy: Option<String> = None;
        let mut body: Option<String> = None;
        let mut response_type: Option<String> = None;
        let mut errors: Vec<String> = Vec::new();

        loop {
            self.skip_newlines();
            match self.current_token().clone() {
                Token::RBrace | Token::Eof => break,

                Token::Method => {
                    self.advance();
                    method = Some(self.parse_http_method()?);
                }
                Token::Path => {
                    self.advance();
                    path = Some(self.expect_path()?);
                }
                Token::Auth => {
                    self.advance();
                    auth = self.parse_auth_requirement()?;
                }
                Token::Policy => {
                    self.advance();
                    policy = Some(self.expect_string()?);
                }
                Token::Body => {
                    self.advance();
                    body = Some(self.expect_ident()?);
                }
                Token::Response => {
                    self.advance();
                    response_type = Some(self.expect_ident()?);
                }
                Token::Errors => {
                    self.advance();
                    errors = self.parse_errors_list()?;
                }
                tok => {
                    return Err(DslError::Parse {
                        line: self.current_line(),
                        message: format!(
                            "expected endpoint field keyword (method, path, auth, policy, body, response, errors), found {:?}",
                            tok
                        ),
                    })
                }
            }
        }
        self.expect(Token::RBrace)?;

        let method = method.ok_or_else(|| DslError::Parse {
            line: self.current_line(),
            message: format!("endpoint '{name}' is missing 'method'"),
        })?;
        let path = path.ok_or_else(|| DslError::Parse {
            line: self.current_line(),
            message: format!("endpoint '{name}' is missing 'path'"),
        })?;
        let response_type = response_type.ok_or_else(|| DslError::Parse {
            line: self.current_line(),
            message: format!("endpoint '{name}' is missing 'response'"),
        })?;

        Ok(EndpointDef { name, method, path, auth, policy, body, response_type, errors })
    }

    fn parse_http_method(&mut self) -> Result<HttpMethod, DslError> {
        match self.current_token().clone() {
            Token::Get => { self.advance(); Ok(HttpMethod::Get) }
            Token::Post => { self.advance(); Ok(HttpMethod::Post) }
            Token::Put => { self.advance(); Ok(HttpMethod::Put) }
            Token::Patch => { self.advance(); Ok(HttpMethod::Patch) }
            Token::Delete => { self.advance(); Ok(HttpMethod::Delete) }
            tok => Err(DslError::Parse {
                line: self.current_line(),
                message: format!("expected HTTP method (GET, POST, PUT, PATCH, DELETE), found {:?}", tok),
            }),
        }
    }

    fn parse_auth_requirement(&mut self) -> Result<AuthRequirement, DslError> {
        match self.current_token().clone() {
            Token::Required => { self.advance(); Ok(AuthRequirement::Required) }
            Token::Optional => { self.advance(); Ok(AuthRequirement::Optional) }
            Token::Ident(s) if s == "none" => { self.advance(); Ok(AuthRequirement::None) }
            tok => Err(DslError::Parse {
                line: self.current_line(),
                message: format!("expected auth value (required, optional, none), found {:?}", tok),
            }),
        }
    }

    fn parse_errors_list(&mut self) -> Result<Vec<String>, DslError> {
        self.expect(Token::LBracket)?;
        let mut errors = Vec::new();
        loop {
            self.skip_newlines();
            match self.current_token().clone() {
                Token::RBracket | Token::Eof => break,
                Token::Comma => { self.advance(); }
                Token::Ident(s) => {
                    self.advance();
                    errors.push(s);
                }
                tok => {
                    return Err(DslError::Parse {
                        line: self.current_line(),
                        message: format!("expected error name or ']', found {:?}", tok),
                    })
                }
            }
        }
        self.expect(Token::RBracket)?;
        Ok(errors)
    }

    // ── helpers ────────────────────────────────────────────────────────────

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
                message: format!("expected {:?}, found {:?}", expected, self.current_token()),
            })
        }
    }

    /// Expect a plain `Token::Ident`.
    fn expect_ident(&mut self) -> Result<String, DslError> {
        match self.current_token().clone() {
            Token::Ident(s) => {
                self.advance();
                Ok(s)
            }
            other => Err(DslError::Parse {
                line: self.current_line(),
                message: format!("expected identifier, found {:?}", other),
            }),
        }
    }

    /// Accept an identifier **or** any keyword token as a plain string.
    /// This is needed for field names that could collide with reserved words,
    /// and for type names like `UUID` which are `Ident` tokens.
    fn expect_any_ident(&mut self) -> Result<String, DslError> {
        match self.current_token().clone() {
            Token::Ident(s) => { self.advance(); Ok(s) }
            Token::Model => { self.advance(); Ok("model".into()) }
            Token::Request => { self.advance(); Ok("request".into()) }
            Token::Endpoint => { self.advance(); Ok("endpoint".into()) }
            Token::Enum => { self.advance(); Ok("enum".into()) }
            Token::Event => { self.advance(); Ok("event".into()) }
            Token::Method => { self.advance(); Ok("method".into()) }
            Token::Path => { self.advance(); Ok("path".into()) }
            Token::Auth => { self.advance(); Ok("auth".into()) }
            Token::Body => { self.advance(); Ok("body".into()) }
            Token::Response => { self.advance(); Ok("response".into()) }
            Token::Errors => { self.advance(); Ok("errors".into()) }
            Token::Policy => { self.advance(); Ok("policy".into()) }
            Token::Required => { self.advance(); Ok("required".into()) }
            Token::Optional => { self.advance(); Ok("optional".into()) }
            Token::Get => { self.advance(); Ok("GET".into()) }
            Token::Post => { self.advance(); Ok("POST".into()) }
            Token::Put => { self.advance(); Ok("PUT".into()) }
            Token::Patch => { self.advance(); Ok("PATCH".into()) }
            Token::Delete => { self.advance(); Ok("DELETE".into()) }
            other => Err(DslError::Parse {
                line: self.current_line(),
                message: format!("expected identifier, found {:?}", other),
            }),
        }
    }

    fn expect_path(&mut self) -> Result<String, DslError> {
        match self.current_token().clone() {
            Token::PathLit(s) => {
                self.advance();
                Ok(s)
            }
            other => Err(DslError::Parse {
                line: self.current_line(),
                message: format!("expected path literal (starting with /), found {:?}", other),
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
                message: format!("expected string literal, found {:?}", other),
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
            "model User {\n  id UUID @primary\n  email Email @unique\n  name String\n}",
        );
        let model = &schema.models[0];
        assert_eq!(model.fields.len(), 3);
        assert_eq!(model.fields[0].name, "id");
        assert_eq!(model.fields[0].ty, FieldType::Uuid);
        assert!(model.fields[0].directives.contains(&Directive::Primary));
    }

    #[test]
    fn parses_model_with_max_directive() {
        let schema = parse("model User {\n  email String @unique @max(255)\n}");
        let field = &schema.models[0].fields[0];
        assert!(field.directives.contains(&Directive::Unique));
        assert!(field.directives.contains(&Directive::MaxLen(255)));
    }

    #[test]
    fn parses_model_with_default_directive() {
        let schema = parse("model User {\n  role UserRole @default(USER)\n}");
        let field = &schema.models[0].fields[0];
        assert!(field.directives.contains(&Directive::Default("USER".into())));
    }

    #[test]
    fn parses_request() {
        let schema = parse(
            "request CreateUserRequest {\n  email String @email\n  name String @min(2) @max(100)\n}",
        );
        assert_eq!(schema.requests.len(), 1);
        let req = &schema.requests[0];
        assert_eq!(req.name, "CreateUserRequest");
        assert_eq!(req.fields.len(), 2);
        assert!(req.fields[0].directives.contains(&Directive::Email));
        assert!(req.fields[1].directives.contains(&Directive::MinLen(2)));
        assert!(req.fields[1].directives.contains(&Directive::MaxLen(100)));
    }

    #[test]
    fn parses_enum() {
        let schema = parse("enum UserRole {\n  USER\n  ADMIN\n  SUPER_ADMIN\n}");
        assert_eq!(schema.enums.len(), 1);
        let e = &schema.enums[0];
        assert_eq!(e.name, "UserRole");
        assert_eq!(e.variants, vec!["USER", "ADMIN", "SUPER_ADMIN"]);
    }

    #[test]
    fn parses_endpoint_block() {
        let schema = parse(
            "model User { id UUID @primary }\n\
             endpoint GetUser {\n  method GET\n  path /users/{id}\n  auth required\n  response User\n}",
        );
        assert_eq!(schema.endpoints.len(), 1);
        let ep = &schema.endpoints[0];
        assert_eq!(ep.name, "GetUser");
        assert_eq!(ep.method, HttpMethod::Get);
        assert_eq!(ep.path, "/users/{id}");
        assert_eq!(ep.auth, AuthRequirement::Required);
        assert_eq!(ep.response_type, "User");
    }

    #[test]
    fn parses_endpoint_with_body_and_errors() {
        let schema = parse(
            "model User { id UUID @primary }\n\
             request CreateUserRequest { email String }\n\
             endpoint CreateUser {\n  method POST\n  path /users\n  auth required\n  body CreateUserRequest\n  response User\n  errors [EmailTaken, ValidationError]\n}",
        );
        let ep = &schema.endpoints[0];
        assert_eq!(ep.body, Some("CreateUserRequest".into()));
        assert_eq!(ep.errors, vec!["EmailTaken", "ValidationError"]);
    }

    #[test]
    fn parses_endpoint_with_policy() {
        let schema = parse(
            "model User { id UUID @primary }\n\
             endpoint CreateUser {\n  method POST\n  path /users\n  auth required\n  policy \"user.role != BANNED\"\n  response User\n}",
        );
        let ep = &schema.endpoints[0];
        assert_eq!(ep.policy, Some("user.role != BANNED".into()));
    }

    #[test]
    fn parses_optional_field() {
        let schema = parse("model User {\n  bio String?\n}");
        let field = &schema.models[0].fields[0];
        assert!(field.optional);
    }

    #[test]
    fn parses_array_field() {
        let schema = parse("model Post {\n  tags String[]\n}");
        let field = &schema.models[0].fields[0];
        assert_eq!(field.ty, FieldType::Array(Box::new(FieldType::String)));
    }

    #[test]
    fn parses_full_schema() {
        let src = r#"
enum UserRole {
  USER
  ADMIN
  SUPER_ADMIN
}

model User {
  id      UUID      @primary @auto
  email   String    @unique @max(255)
  name    String    @max(100)
  role    UserRole  @default(USER)
  created Timestamp @auto
}

request CreateUserRequest {
  email String @email
  name  String @min(2) @max(100)
}

endpoint CreateUser {
  method   POST
  path     /users
  auth     required
  policy   "user.role != BANNED"
  body     CreateUserRequest
  response User
  errors   [EmailTaken, ValidationError]
}

endpoint GetUser {
  method   GET
  path     /users/{id}
  auth     required
  response User
}
"#;
        let schema = parse(src);
        assert_eq!(schema.enums.len(), 1);
        assert_eq!(schema.models.len(), 1);
        assert_eq!(schema.requests.len(), 1);
        assert_eq!(schema.endpoints.len(), 2);
    }
}
