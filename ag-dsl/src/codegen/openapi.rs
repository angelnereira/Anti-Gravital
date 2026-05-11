use serde_json::{json, Value};

use crate::ast::{EndpointDef, FieldType, HttpMethod, ModelDef, SchemaFile};
use crate::codegen::GeneratedFile;
use crate::error::DslError;

pub fn generate(schema: &SchemaFile) -> Result<GeneratedFile, DslError> {
    let mut schemas = serde_json::Map::new();
    for model in &schema.models {
        schemas.insert(model.name.clone(), model_to_schema(model));
    }

    let mut paths = serde_json::Map::new();
    for ep in &schema.endpoints {
        let axum_path = convert_path(&ep.path);
        let entry = paths.entry(axum_path).or_insert_with(|| json!({}));
        let method = method_key(ep.method);
        entry[method] = endpoint_to_operation(ep);
    }

    let doc = json!({
        "openapi": "3.1.0",
        "info": {
            "title": "Anti-Gravital API",
            "version": "0.1.0",
        },
        "paths": paths,
        "components": {
            "schemas": schemas,
        },
    });

    let content = serde_json::to_string_pretty(&doc)
        .map_err(|e| DslError::Codegen(e.to_string()))?;

    Ok(GeneratedFile {
        path: "openapi.json".into(),
        content,
    })
}

fn model_to_schema(model: &ModelDef) -> Value {
    let mut properties = serde_json::Map::new();
    let mut required = Vec::new();

    for field in &model.fields {
        properties.insert(field.name.clone(), field_type_to_schema(&field.ty));
        if !field.optional {
            required.push(field.name.clone());
        }
    }

    json!({
        "type": "object",
        "properties": properties,
        "required": required,
    })
}

fn field_type_to_schema(ty: &FieldType) -> Value {
    match ty {
        FieldType::Uuid => json!({"type": "string", "format": "uuid"}),
        FieldType::Email => json!({"type": "string", "format": "email"}),
        FieldType::String => json!({"type": "string"}),
        FieldType::Int => json!({"type": "integer", "format": "int64"}),
        FieldType::Float => json!({"type": "number", "format": "double"}),
        FieldType::Bool => json!({"type": "boolean"}),
        FieldType::Timestamp => json!({"type": "string", "format": "date-time"}),
        FieldType::Json => json!({}),
        FieldType::Model(name) => json!({"$ref": format!("#/components/schemas/{name}")}),
        FieldType::Array(inner) => json!({"type": "array", "items": field_type_to_schema(inner)}),
    }
}

fn endpoint_to_operation(ep: &EndpointDef) -> Value {
    json!({
        "responses": {
            "200": {
                "description": "Success",
                "content": {
                    "application/json": {
                        "schema": {
                            "$ref": format!("#/components/schemas/{}", ep.response_type)
                        }
                    }
                }
            }
        }
    })
}

fn convert_path(path: &str) -> String {
    // Convert Axum-style `{param}` to OpenAPI-style `{param}` (same format, no-op).
    path.to_owned()
}

fn method_key(method: HttpMethod) -> &'static str {
    match method {
        HttpMethod::Get => "get",
        HttpMethod::Post => "post",
        HttpMethod::Put => "put",
        HttpMethod::Patch => "patch",
        HttpMethod::Delete => "delete",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{FieldDef, ModelDef, SchemaFile};

    #[test]
    fn generates_valid_json() {
        let schema = SchemaFile {
            models: vec![ModelDef {
                name: "User".into(),
                fields: vec![FieldDef {
                    name: "id".into(),
                    ty: FieldType::Uuid,
                    directives: vec![],
                    optional: false,
                }],
            }],
            endpoints: vec![],
        };
        let file = generate(&schema).unwrap();
        let parsed: Value = serde_json::from_str(&file.content).unwrap();
        assert_eq!(parsed["openapi"], "3.1.0");
        assert!(parsed["components"]["schemas"]["User"].is_object());
    }
}
