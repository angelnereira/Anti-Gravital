use crate::ast::{AuthRequirement, EndpointDef, EnumDef, FieldDef, FieldType, HttpMethod, ModelDef, RequestDef, SchemaFile};
use crate::codegen::GeneratedFile;
use crate::error::DslError;

pub fn generate(schema: &SchemaFile) -> Result<GeneratedFile, DslError> {
    let content = generate_yaml(schema);
    Ok(GeneratedFile {
        path: "openapi.yaml".into(),
        content,
    })
}

fn generate_yaml(schema: &SchemaFile) -> String {
    let mut out = String::new();

    out.push_str("openapi: \"3.1.0\"\n");
    out.push_str("info:\n");
    out.push_str("  title: Anti-Gravital API\n");
    out.push_str("  version: \"0.1.0\"\n");

    // ── paths ──────────────────────────────────────────────────────────────
    if schema.endpoints.is_empty() {
        out.push_str("paths: {}\n");
    } else {
        out.push_str("paths:\n");

        // Group endpoints by path while preserving insertion order
        let mut path_groups: Vec<(String, Vec<&EndpointDef>)> = Vec::new();
        for ep in &schema.endpoints {
            if let Some(group) = path_groups.iter_mut().find(|(p, _)| p == &ep.path) {
                group.1.push(ep);
            } else {
                path_groups.push((ep.path.clone(), vec![ep]));
            }
        }

        for (path, eps) in &path_groups {
            out.push_str(&format!("  {}:\n", path));
            for ep in eps {
                let method = method_key(ep.method);
                let op_id = endpoint_name_to_snake(&ep.name);
                out.push_str(&format!("    {}:\n", method));
                out.push_str(&format!("      operationId: {}\n", op_id));
                out.push_str(&format!("      summary: {}\n", ep.name));

                // Security
                match ep.auth {
                    AuthRequirement::Required => {
                        out.push_str("      security:\n");
                        out.push_str("        - bearerAuth: []\n");
                    }
                    AuthRequirement::Optional => {
                        out.push_str("      security:\n");
                        out.push_str("        - {}\n");
                        out.push_str("        - bearerAuth: []\n");
                    }
                    AuthRequirement::None => {}
                }

                // Request body
                if let Some(ref body) = ep.body {
                    out.push_str("      requestBody:\n");
                    out.push_str("        required: true\n");
                    out.push_str("        content:\n");
                    out.push_str("          application/json:\n");
                    out.push_str("            schema:\n");
                    out.push_str(&format!(
                        "              $ref: \"#/components/schemas/{}\"\n",
                        body
                    ));
                }

                // Responses
                out.push_str("      responses:\n");
                out.push_str("        \"200\":\n");
                out.push_str("          description: Success\n");
                out.push_str("          content:\n");
                out.push_str("            application/json:\n");
                out.push_str("              schema:\n");
                out.push_str(&format!(
                    "                $ref: \"#/components/schemas/{}\"\n",
                    ep.response_type
                ));

                // Error responses
                for err in &ep.errors {
                    let (code, desc) = error_to_status(err);
                    out.push_str(&format!("        \"{}\":\n", code));
                    out.push_str(&format!("          description: {}\n", desc));
                }
            }
        }
    }

    // ── components/schemas ─────────────────────────────────────────────────
    let has_schemas = !schema.models.is_empty()
        || !schema.requests.is_empty()
        || !schema.enums.is_empty();

    if has_schemas {
        out.push_str("components:\n");
        out.push_str("  schemas:\n");

        for enum_def in &schema.enums {
            out.push_str(&generate_enum_schema(enum_def));
        }
        for model in &schema.models {
            out.push_str(&generate_model_schema(model));
        }
        for request in &schema.requests {
            out.push_str(&generate_request_schema(request));
        }
    }

    out
}

fn generate_enum_schema(enum_def: &EnumDef) -> String {
    let mut out = String::new();
    out.push_str(&format!("    {}:\n", enum_def.name));
    out.push_str("      type: string\n");
    out.push_str("      enum:\n");
    for variant in &enum_def.variants {
        out.push_str(&format!("        - {}\n", variant));
    }
    out
}

fn generate_model_schema(model: &ModelDef) -> String {
    generate_object_schema(&model.name, &model.fields)
}

fn generate_request_schema(request: &RequestDef) -> String {
    generate_object_schema(&request.name, &request.fields)
}

fn generate_object_schema(name: &str, fields: &[FieldDef]) -> String {
    let mut out = String::new();
    out.push_str(&format!("    {}:\n", name));
    out.push_str("      type: object\n");

    let required_fields: Vec<&str> = fields
        .iter()
        .filter(|f| !f.optional)
        .map(|f| f.name.as_str())
        .collect();

    if !required_fields.is_empty() {
        out.push_str("      required:\n");
        for f in &required_fields {
            out.push_str(&format!("        - {}\n", f));
        }
    }

    if !fields.is_empty() {
        out.push_str("      properties:\n");
        for field in fields {
            out.push_str(&format!("        {}:\n", field.name));
            let schema = field_type_to_yaml_schema(&field.ty);
            for line in schema.lines() {
                out.push_str(&format!("          {}\n", line));
            }
        }
    }

    out
}

fn field_type_to_yaml_schema(ty: &FieldType) -> String {
    match ty {
        FieldType::Uuid => "type: string\nformat: uuid".into(),
        FieldType::Email => "type: string\nformat: email".into(),
        FieldType::String => "type: string".into(),
        FieldType::Int => "type: integer\nformat: int64".into(),
        FieldType::Float => "type: number\nformat: double".into(),
        FieldType::Bool => "type: boolean".into(),
        FieldType::Timestamp => "type: string\nformat: date-time".into(),
        FieldType::Json => "type: object\nadditionalProperties: true".into(),
        FieldType::Model(name) => format!("$ref: \"#/components/schemas/{}\"", name),
        FieldType::Array(inner) => {
            let items = field_type_to_yaml_schema(inner);
            let mut out = String::from("type: array\nitems:\n");
            for line in items.lines() {
                out.push_str(&format!("  {}\n", line));
            }
            out.trim_end().to_owned()
        }
    }
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

fn endpoint_name_to_snake(name: &str) -> String {
    let mut out = String::new();
    for (i, ch) in name.chars().enumerate() {
        if ch.is_uppercase() && i > 0 {
            out.push('_');
        }
        out.push(ch.to_ascii_lowercase());
    }
    out
}

/// Map common error names to HTTP status codes.
fn error_to_status(err: &str) -> (&'static str, &'static str) {
    match err {
        "EmailTaken" | "Conflict" | "AlreadyExists" => ("409", "Conflict"),
        "ValidationError" | "BadRequest" | "InvalidInput" => ("400", "Bad Request"),
        "NotFound" => ("404", "Not Found"),
        "Unauthorized" => ("401", "Unauthorized"),
        "Forbidden" => ("403", "Forbidden"),
        "RateLimited" => ("429", "Too Many Requests"),
        _ => ("400", "Error"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{AuthRequirement, EndpointDef, FieldDef, FieldType, HttpMethod, ModelDef, RequestDef, SchemaFile};

    fn base_schema() -> SchemaFile {
        SchemaFile {
            models: vec![ModelDef {
                name: "User".into(),
                fields: vec![FieldDef {
                    name: "id".into(),
                    ty: FieldType::Uuid,
                    directives: vec![],
                    optional: false,
                }],
            }],
            requests: vec![],
            endpoints: vec![],
            enums: vec![],
        }
    }

    #[test]
    fn generates_valid_yaml_header() {
        let schema = base_schema();
        let file = generate(&schema).unwrap();
        assert!(file.content.contains("openapi: \"3.1.0\""));
        assert!(file.content.contains("title: Anti-Gravital API"));
    }

    #[test]
    fn generates_model_schema() {
        let schema = base_schema();
        let file = generate(&schema).unwrap();
        assert!(file.content.contains("User:"), "missing User schema");
        assert!(file.content.contains("format: uuid"), "missing uuid format");
    }

    #[test]
    fn generates_endpoint_path() {
        let mut schema = base_schema();
        schema.endpoints.push(EndpointDef {
            name: "GetUser".into(),
            method: HttpMethod::Get,
            path: "/users/{id}".into(),
            auth: AuthRequirement::Required,
            policy: None,
            body: None,
            response_type: "User".into(),
            errors: vec![],
        });
        let file = generate(&schema).unwrap();
        assert!(file.content.contains("/users/{id}:"));
        assert!(file.content.contains("operationId: get_user"));
        assert!(file.content.contains("bearerAuth"));
    }

    #[test]
    fn generates_post_with_body() {
        let mut schema = base_schema();
        schema.requests.push(RequestDef {
            name: "CreateUserRequest".into(),
            fields: vec![FieldDef {
                name: "email".into(),
                ty: FieldType::String,
                directives: vec![],
                optional: false,
            }],
        });
        schema.endpoints.push(EndpointDef {
            name: "CreateUser".into(),
            method: HttpMethod::Post,
            path: "/users".into(),
            auth: AuthRequirement::Required,
            policy: None,
            body: Some("CreateUserRequest".into()),
            response_type: "User".into(),
            errors: vec!["EmailTaken".into(), "ValidationError".into()],
        });
        let file = generate(&schema).unwrap();
        assert!(file.content.contains("requestBody:"));
        assert!(file.content.contains("CreateUserRequest"));
        assert!(file.content.contains("\"409\""));
        assert!(file.content.contains("\"400\""));
    }

    #[test]
    fn yaml_output_path_is_correct() {
        let schema = base_schema();
        let file = generate(&schema).unwrap();
        assert_eq!(file.path, "openapi.yaml");
    }
}
