use crate::ast::{Directive, EndpointDef, EnumDef, FieldDef, FieldType, HttpMethod, ModelDef, RequestDef, SchemaFile};
use crate::codegen::GeneratedFile;
use crate::error::DslError;

pub fn generate(schema: &SchemaFile) -> Result<Vec<GeneratedFile>, DslError> {
    let models_rs = generate_models_file(schema);
    let validators_rs = generate_validators_file(schema);
    let stubs_rs = generate_handler_stubs_file(schema);

    Ok(vec![
        GeneratedFile { path: "src/models.rs".into(), content: models_rs },
        GeneratedFile { path: "src/validators.rs".into(), content: validators_rs },
        GeneratedFile { path: "src/handlers/stubs.rs".into(), content: stubs_rs },
    ])
}

// ── models.rs ──────────────────────────────────────────────────────────────

fn generate_models_file(schema: &SchemaFile) -> String {
    let mut out = String::new();
    out.push_str("// GENERATED — do not edit. Regenerate with `ag generate`.\n\n");
    out.push_str("use serde::{Deserialize, Serialize};\n");
    out.push_str("use uuid::Uuid;\n\n");

    for enum_def in &schema.enums {
        out.push_str(&generate_enum(enum_def));
        out.push('\n');
    }

    for model in &schema.models {
        out.push_str(&generate_model(model));
        out.push('\n');
    }

    for request in &schema.requests {
        out.push_str(&generate_request(request));
        out.push('\n');
    }

    out
}

fn generate_enum(enum_def: &EnumDef) -> String {
    let mut out = String::new();
    out.push_str("#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]\n");
    out.push_str("#[serde(rename_all = \"SCREAMING_SNAKE_CASE\")]\n");
    out.push_str(&format!("pub enum {} {{\n", enum_def.name));
    for variant in &enum_def.variants {
        // Convert SCREAMING_SNAKE_CASE to PascalCase for Rust identifiers
        let pascal = screaming_to_pascal(variant);
        out.push_str(&format!("    {},\n", pascal));
    }
    out.push_str("}\n");
    out
}

fn generate_model(model: &ModelDef) -> String {
    let mut out = String::new();
    out.push_str("#[derive(Debug, Clone, Serialize, Deserialize)]\n");
    out.push_str(&format!("pub struct {} {{\n", model.name));
    for field in &model.fields {
        out.push_str(&generate_field(field));
    }
    out.push_str("}\n");
    out
}

fn generate_request(request: &RequestDef) -> String {
    let mut out = String::new();
    out.push_str("#[derive(Debug, Clone, Serialize, Deserialize)]\n");
    out.push_str(&format!("pub struct {} {{\n", request.name));
    for field in &request.fields {
        out.push_str(&generate_field(field));
    }
    out.push_str("}\n");
    out
}

fn generate_field(field: &FieldDef) -> String {
    let rust_type = field_type_to_rust(&field.ty);
    let ty = if field.optional {
        format!("Option<{rust_type}>")
    } else {
        rust_type
    };
    format!("    pub {}: {},\n", field.name, ty)
}

pub fn field_type_to_rust(ty: &FieldType) -> String {
    match ty {
        FieldType::Uuid => "Uuid".into(),
        FieldType::Email => "String".into(),
        FieldType::String => "String".into(),
        FieldType::Int => "i64".into(),
        FieldType::Float => "f64".into(),
        FieldType::Bool => "bool".into(),
        FieldType::Timestamp => "chrono::DateTime<chrono::Utc>".into(),
        FieldType::Json => "serde_json::Value".into(),
        FieldType::Model(name) => name.clone(),
        FieldType::Array(inner) => format!("Vec<{}>", field_type_to_rust(inner)),
    }
}

// ── validators.rs ──────────────────────────────────────────────────────────

fn generate_validators_file(schema: &SchemaFile) -> String {
    let mut out = String::new();
    out.push_str("// GENERATED — do not edit. Regenerate with `ag generate`.\n\n");

    if schema.requests.is_empty() {
        out.push_str("// No request types defined — no validators generated.\n");
        return out;
    }

    // Build import list
    let request_names: Vec<&str> = schema.requests.iter().map(|r| r.name.as_str()).collect();
    out.push_str(&format!("use crate::models::{{{}}};\n\n", request_names.join(", ")));

    for request in &schema.requests {
        out.push_str(&generate_validator(request));
        out.push('\n');
    }

    out
}

fn generate_validator(request: &RequestDef) -> String {
    let fn_name = format!("validate_{}", to_snake_case(&request.name));
    let mut out = String::new();
    out.push_str(&format!(
        "pub fn {fn_name}(req: &{}) -> Result<(), Vec<String>> {{\n",
        request.name
    ));
    out.push_str("    let mut errors = Vec::new();\n");

    for field in &request.fields {
        for directive in &field.directives {
            match directive {
                Directive::Email => {
                    out.push_str(&format!(
                        "    if req.{}.is_empty() {{ errors.push(\"{}: required\".into()); }}\n",
                        field.name, field.name
                    ));
                }
                Directive::MinLen(n) => {
                    out.push_str(&format!(
                        "    if req.{}.len() < {n} {{ errors.push(\"{}: minimum length is {n}\".into()); }}\n",
                        field.name, field.name
                    ));
                }
                Directive::MaxLen(n) => {
                    out.push_str(&format!(
                        "    if req.{}.len() > {n} {{ errors.push(\"{}: maximum length is {n}\".into()); }}\n",
                        field.name, field.name
                    ));
                }
                _ => {}
            }
        }
    }

    out.push_str("    if errors.is_empty() { Ok(()) } else { Err(errors) }\n");
    out.push_str("}\n");
    out
}

// ── handlers/stubs.rs ──────────────────────────────────────────────────────

fn generate_handler_stubs_file(schema: &SchemaFile) -> String {
    let mut out = String::new();
    out.push_str("// GENERATED — do not edit. Fill in the body of each handler.\n\n");
    out.push_str("use axum::{extract::State, Json};\n");

    // Collect all response types and body types used
    let mut model_imports: Vec<String> = Vec::new();
    for ep in &schema.endpoints {
        if !model_imports.contains(&ep.response_type) {
            model_imports.push(ep.response_type.clone());
        }
        if let Some(ref body) = ep.body {
            if !model_imports.contains(body) {
                model_imports.push(body.clone());
            }
        }
    }
    if !model_imports.is_empty() {
        out.push_str(&format!("use crate::models::{{{}}};\n", model_imports.join(", ")));
    }

    out.push('\n');

    for ep in &schema.endpoints {
        out.push_str(&generate_handler_stub(ep));
        out.push('\n');
    }

    out
}

fn generate_handler_stub(ep: &EndpointDef) -> String {
    let fn_name = endpoint_name_to_fn(&ep.name);
    let response = &ep.response_type;
    let mut out = String::new();

    out.push_str(&format!("pub async fn {fn_name}(\n"));

    if let Some(ref body) = ep.body {
        out.push_str(&format!("    // ValidatedBody(req): ValidatedBody<{body}>,\n"));
    }

    match ep.method {
        HttpMethod::Get | HttpMethod::Delete => {}
        _ => {}
    }

    // Path params comment
    let path_params: Vec<&str> = ep
        .path
        .split('/')
        .filter(|s| s.starts_with('{') && s.ends_with('}'))
        .map(|s| &s[1..s.len() - 1])
        .collect();
    if !path_params.is_empty() {
        out.push_str(&format!(
            "    // Path(({})): Path<({})>,\n",
            path_params.join(", "),
            path_params.iter().map(|_| "String").collect::<Vec<_>>().join(", ")
        ));
    }

    out.push_str(&format!(
        ") -> Result<Json<{response}>, ag_core::core::error::AgError> {{\n"
    ));
    out.push_str(&format!("    todo!(\"implement {fn_name}\")\n"));
    out.push_str("}\n");
    out
}

// ── utilities ──────────────────────────────────────────────────────────────

/// Convert `PascalCase` to `snake_case`.
fn to_snake_case(s: &str) -> String {
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() && i > 0 {
            out.push('_');
        }
        out.push(ch.to_ascii_lowercase());
    }
    out
}

/// Convert endpoint PascalCase name to snake_case function name.
fn endpoint_name_to_fn(name: &str) -> String {
    to_snake_case(name)
}

/// Convert `SCREAMING_SNAKE_CASE` to `PascalCase`.
fn screaming_to_pascal(s: &str) -> String {
    s.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => {
                    let upper = first.to_uppercase().to_string();
                    upper + &chars.as_str().to_lowercase()
                }
            }
        })
        .collect::<Vec<_>>()
        .join("")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Directive, EnumDef, FieldDef, FieldType, ModelDef, RequestDef};

    #[test]
    fn generates_struct_from_model() {
        let model = ModelDef {
            name: "User".into(),
            fields: vec![
                FieldDef {
                    name: "id".into(),
                    ty: FieldType::Uuid,
                    directives: vec![Directive::Primary],
                    optional: false,
                },
                FieldDef {
                    name: "email".into(),
                    ty: FieldType::Email,
                    directives: vec![],
                    optional: false,
                },
            ],
        };
        let src = generate_model(&model);
        assert!(src.contains("pub struct User"), "missing struct User");
        assert!(src.contains("pub id: Uuid"), "missing id field");
        assert!(src.contains("pub email: String"), "missing email field");
    }

    #[test]
    fn generates_enum() {
        let enum_def = EnumDef {
            name: "UserRole".into(),
            variants: vec!["USER".into(), "ADMIN".into(), "SUPER_ADMIN".into()],
        };
        let src = generate_enum(&enum_def);
        assert!(src.contains("pub enum UserRole"));
        assert!(src.contains("User,"));
        assert!(src.contains("Admin,"));
        assert!(src.contains("SuperAdmin,"));
        assert!(src.contains("SCREAMING_SNAKE_CASE"));
    }

    #[test]
    fn generates_request_struct() {
        let request = RequestDef {
            name: "CreateUserRequest".into(),
            fields: vec![FieldDef {
                name: "email".into(),
                ty: FieldType::String,
                directives: vec![Directive::Email],
                optional: false,
            }],
        };
        let src = generate_request(&request);
        assert!(src.contains("pub struct CreateUserRequest"));
        assert!(src.contains("pub email: String"));
    }

    #[test]
    fn generates_validator_with_min_max() {
        let request = RequestDef {
            name: "CreateUserRequest".into(),
            fields: vec![
                FieldDef {
                    name: "email".into(),
                    ty: FieldType::String,
                    directives: vec![Directive::Email],
                    optional: false,
                },
                FieldDef {
                    name: "name".into(),
                    ty: FieldType::String,
                    directives: vec![Directive::MinLen(2), Directive::MaxLen(100)],
                    optional: false,
                },
            ],
        };
        let src = generate_validator(&request);
        assert!(src.contains("validate_create_user_request"));
        assert!(src.contains("minimum length is 2"));
        assert!(src.contains("maximum length is 100"));
    }

    #[test]
    fn screaming_to_pascal_conversion() {
        assert_eq!(screaming_to_pascal("SUPER_ADMIN"), "SuperAdmin");
        assert_eq!(screaming_to_pascal("USER"), "User");
        assert_eq!(screaming_to_pascal("ADMIN"), "Admin");
    }

    #[test]
    fn to_snake_case_conversion() {
        assert_eq!(to_snake_case("CreateUser"), "create_user");
        assert_eq!(to_snake_case("GetUser"), "get_user");
        assert_eq!(to_snake_case("ListUsers"), "list_users");
    }

    #[test]
    fn optional_field_uses_option() {
        let model = ModelDef {
            name: "User".into(),
            fields: vec![FieldDef {
                name: "bio".into(),
                ty: FieldType::String,
                directives: vec![],
                optional: true,
            }],
        };
        let src = generate_model(&model);
        assert!(src.contains("pub bio: Option<String>"));
    }
}
