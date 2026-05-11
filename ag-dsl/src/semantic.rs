use std::collections::HashSet;

use crate::ast::{EndpointDef, FieldType, SchemaFile};
use crate::error::DslError;

/// Validates semantic constraints that the parser cannot enforce:
///
/// - Model names must be unique.
/// - Endpoint paths must be unique per (method, path) pair.
/// - Fields with type `Model(name)` must reference a declared model.
/// - Array element types are recursively validated.
pub fn check(schema: &SchemaFile) -> Result<(), DslError> {
    let model_names: HashSet<&str> = schema.models.iter().map(|m| m.name.as_str()).collect();

    // Duplicate model names
    if model_names.len() != schema.models.len() {
        return Err(DslError::Semantic("duplicate model name".into()));
    }

    // Field references
    for model in &schema.models {
        for field in &model.fields {
            check_field_type(&field.ty, &model_names, &model.name, &field.name)?;
        }
    }

    // Duplicate endpoint (method, path) pairs
    let mut endpoints: HashSet<String> = HashSet::new();
    for ep in &schema.endpoints {
        let key = format!("{} {}", ep.method, ep.path);
        if !endpoints.insert(key.clone()) {
            return Err(DslError::Semantic(format!("duplicate endpoint: {key}")));
        }
        check_endpoint(ep, &model_names)?;
    }

    Ok(())
}

fn check_field_type(
    ty: &FieldType,
    model_names: &HashSet<&str>,
    model: &str,
    field: &str,
) -> Result<(), DslError> {
    match ty {
        FieldType::Model(name) => {
            if !model_names.contains(name.as_str()) {
                return Err(DslError::Semantic(format!(
                    "model '{model}' field '{field}' references unknown type '{name}'"
                )));
            }
        }
        FieldType::Array(inner) => check_field_type(inner, model_names, model, field)?,
        _ => {}
    }
    Ok(())
}

fn check_endpoint(ep: &EndpointDef, model_names: &HashSet<&str>) -> Result<(), DslError> {
    if !model_names.contains(ep.response_type.as_str()) {
        return Err(DslError::Semantic(format!(
            "endpoint '{} {}' references unknown response type '{}'",
            ep.method, ep.path, ep.response_type
        )));
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

    #[test]
    fn valid_schema_passes() {
        let schema = parse(
            "model User {\n  id: UUID @primary\n}\nendpoint GET /users/{id} -> User",
        );
        assert!(check(&schema).is_ok());
    }

    #[test]
    fn unknown_response_type_fails() {
        let schema = parse("model User {\n  id: UUID\n}\nendpoint GET /posts -> Post");
        assert!(check(&schema).is_err());
    }

    #[test]
    fn duplicate_model_fails() {
        let schema = parse("model A {}\nmodel A {}");
        assert!(check(&schema).is_err());
    }
}
