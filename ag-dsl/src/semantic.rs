use std::collections::HashSet;

use crate::ast::{Directive, EndpointDef, FieldType, SchemaFile};
use crate::error::DslError;

/// Validates semantic constraints that the parser cannot enforce:
///
/// - Model names must be unique.
/// - Request names must be unique.
/// - Endpoint names must be unique.
/// - Enum names must be unique.
/// - Fields with type `Model(name)` must reference a declared model or enum.
/// - Array element types are recursively validated.
/// - Endpoint `body` must reference a declared request (or model).
/// - Endpoint `response` must reference a declared model.
/// - `@default(VAL)` on an enum-typed field must be a valid variant of that enum.
pub fn check(schema: &SchemaFile) -> Result<(), DslError> {
    let model_names: HashSet<&str> = schema.models.iter().map(|m| m.name.as_str()).collect();
    let request_names: HashSet<&str> = schema.requests.iter().map(|r| r.name.as_str()).collect();
    let enum_names: HashSet<&str> = schema.enums.iter().map(|e| e.name.as_str()).collect();

    // Known type names = models + enums
    let known_types: HashSet<&str> = model_names.iter().chain(enum_names.iter()).copied().collect();

    // Duplicate model names
    if model_names.len() != schema.models.len() {
        return Err(DslError::Semantic("duplicate model name".into()));
    }

    // Duplicate request names
    if request_names.len() != schema.requests.len() {
        return Err(DslError::Semantic("duplicate request name".into()));
    }

    // Duplicate enum names
    if enum_names.len() != schema.enums.len() {
        return Err(DslError::Semantic("duplicate enum name".into()));
    }

    // Field references in models
    for model in &schema.models {
        for field in &model.fields {
            check_field_type(&field.ty, &known_types, &model.name, &field.name)?;

            // Validate @default(VAL) for enum-typed fields
            if let FieldType::Model(ref type_name) = field.ty {
                if enum_names.contains(type_name.as_str()) {
                    // Find the enum and check the default value is a valid variant
                    for directive in &field.directives {
                        if let Directive::Default(ref val) = directive {
                            let enum_def = schema.enums.iter().find(|e| &e.name == type_name).unwrap();
                            if !enum_def.variants.iter().any(|v| v == val) {
                                return Err(DslError::Semantic(format!(
                                    "model '{}' field '{}': @default('{}') is not a valid variant of enum '{}'",
                                    model.name, field.name, val, type_name
                                )));
                            }
                        }
                    }
                }
            }
        }
    }

    // Field references in requests
    for req in &schema.requests {
        for field in &req.fields {
            check_field_type(&field.ty, &known_types, &req.name, &field.name)?;
        }
    }

    // Duplicate endpoint names
    let mut endpoint_names: HashSet<&str> = HashSet::new();
    for ep in &schema.endpoints {
        if !endpoint_names.insert(ep.name.as_str()) {
            return Err(DslError::Semantic(format!("duplicate endpoint name: '{}'", ep.name)));
        }
        check_endpoint(ep, &model_names, &request_names, &known_types)?;
    }

    Ok(())
}

fn check_field_type(
    ty: &FieldType,
    known_types: &HashSet<&str>,
    context: &str,
    field: &str,
) -> Result<(), DslError> {
    match ty {
        FieldType::Model(name) => {
            if !known_types.contains(name.as_str()) {
                return Err(DslError::Semantic(format!(
                    "'{context}' field '{field}' references unknown type '{name}'"
                )));
            }
        }
        FieldType::Array(inner) => check_field_type(inner, known_types, context, field)?,
        _ => {}
    }
    Ok(())
}

fn check_endpoint(
    ep: &EndpointDef,
    model_names: &HashSet<&str>,
    request_names: &HashSet<&str>,
    known_types: &HashSet<&str>,
) -> Result<(), DslError> {
    // response_type must be a known model
    if !model_names.contains(ep.response_type.as_str()) {
        return Err(DslError::Semantic(format!(
            "endpoint '{}': response type '{}' is not a known model",
            ep.name, ep.response_type
        )));
    }

    // body must be a known request or model (if specified)
    if let Some(ref body) = ep.body {
        if !request_names.contains(body.as_str()) && !known_types.contains(body.as_str()) {
            return Err(DslError::Semantic(format!(
                "endpoint '{}': body type '{}' is not a known request or model",
                ep.name, body
            )));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    fn parse(src: &str) -> SchemaFile {
        let tokens = Lexer::new(src).tokenize().unwrap();
        Parser::new(tokens).parse().unwrap()
    }

    fn check_src(src: &str) -> Result<(), DslError> {
        let schema = parse(src);
        check(&schema)
    }

    #[test]
    fn valid_schema_passes() {
        let result = check_src(
            "model User {\n  id UUID @primary\n}\nendpoint GetUser {\n  method GET\n  path /users/{id}\n  auth required\n  response User\n}",
        );
        assert!(result.is_ok());
    }

    #[test]
    fn unknown_response_type_fails() {
        let result = check_src(
            "model User {\n  id UUID\n}\nendpoint GetPost {\n  method GET\n  path /posts/{id}\n  response Post\n}",
        );
        assert!(result.is_err());
    }

    #[test]
    fn duplicate_model_fails() {
        let schema = parse("model A {}\nmodel A {}");
        assert!(check(&schema).is_err());
    }

    #[test]
    fn duplicate_endpoint_fails() {
        let schema = parse(
            "model User { id UUID }\n\
             endpoint GetUser { method GET  path /users/{id}  response User }\n\
             endpoint GetUser { method GET  path /users/{id}  response User }",
        );
        assert!(check(&schema).is_err());
    }

    #[test]
    fn unknown_field_type_fails() {
        let schema = parse("model Post {\n  author Ghost\n}");
        assert!(check(&schema).is_err());
    }

    #[test]
    fn enum_field_type_passes() {
        let result = check_src(
            "enum UserRole { USER  ADMIN }\nmodel User {\n  role UserRole @default(USER)\n}\n",
        );
        assert!(result.is_ok());
    }

    #[test]
    fn invalid_enum_default_fails() {
        let result = check_src(
            "enum UserRole { USER  ADMIN }\nmodel User {\n  role UserRole @default(BANNED)\n}\n",
        );
        assert!(result.is_err());
    }

    #[test]
    fn valid_request_body_passes() {
        let result = check_src(
            "model User { id UUID }\n\
             request CreateUserRequest { email String }\n\
             endpoint CreateUser {\n  method POST\n  path /users\n  body CreateUserRequest\n  response User\n}",
        );
        assert!(result.is_ok());
    }

    #[test]
    fn unknown_body_type_fails() {
        let result = check_src(
            "model User { id UUID }\n\
             endpoint CreateUser {\n  method POST\n  path /users\n  body GhostRequest\n  response User\n}",
        );
        assert!(result.is_err());
    }
}
