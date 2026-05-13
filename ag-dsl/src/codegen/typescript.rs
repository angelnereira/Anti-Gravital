use crate::ast::{EnumDef, FieldDef, FieldType, ModelDef, RequestDef, SchemaFile};
use crate::codegen::GeneratedFile;
use crate::error::DslError;

pub fn generate(schema: &SchemaFile) -> Result<Vec<GeneratedFile>, DslError> {
    let content = generate_models_ts(schema);
    Ok(vec![GeneratedFile {
        path: "src/models.ts".into(),
        content,
    }])
}

fn generate_models_ts(schema: &SchemaFile) -> String {
    let mut out = String::new();
    out.push_str("// GENERATED — do not edit. Regenerate with `ag generate`.\n\n");

    for enum_def in &schema.enums {
        out.push_str(&generate_enum_type(enum_def));
        out.push('\n');
    }

    for model in &schema.models {
        out.push_str(&generate_interface(model));
        out.push('\n');
    }

    for request in &schema.requests {
        out.push_str(&generate_request_interface(request));
        out.push('\n');
    }

    out
}

fn generate_enum_type(enum_def: &EnumDef) -> String {
    let variants = enum_def
        .variants
        .iter()
        .map(|v| format!("\"{}\"", v))
        .collect::<Vec<_>>()
        .join(" | ");
    format!("export type {} = {};\n", enum_def.name, variants)
}

fn generate_interface(model: &ModelDef) -> String {
    let mut out = String::new();
    out.push_str(&format!("export interface {} {{\n", model.name));
    for field in &model.fields {
        out.push_str(&generate_ts_field(field));
    }
    out.push_str("}\n");
    out
}

fn generate_request_interface(request: &RequestDef) -> String {
    let mut out = String::new();
    out.push_str(&format!("export interface {} {{\n", request.name));
    for field in &request.fields {
        out.push_str(&generate_ts_field(field));
    }
    out.push_str("}\n");
    out
}

fn generate_ts_field(field: &FieldDef) -> String {
    let ts_type = field_type_to_ts(&field.ty);
    let type_comment = field_type_comment(&field.ty);
    let opt = if field.optional { "?" } else { "" };
    if let Some(comment) = type_comment {
        format!("  {}{}: {}; // {}\n", field.name, opt, ts_type, comment)
    } else {
        format!("  {}{}: {};\n", field.name, opt, ts_type)
    }
}

fn field_type_to_ts(ty: &FieldType) -> String {
    match ty {
        FieldType::Uuid => "string".into(),
        FieldType::Email => "string".into(),
        FieldType::String => "string".into(),
        FieldType::Int => "number".into(),
        FieldType::Float => "number".into(),
        FieldType::Bool => "boolean".into(),
        FieldType::Timestamp => "string".into(),
        FieldType::Json => "unknown".into(),
        FieldType::Model(name) => name.clone(),
        FieldType::Array(inner) => format!("{}[]", field_type_to_ts(inner)),
    }
}

fn field_type_comment(ty: &FieldType) -> Option<&'static str> {
    match ty {
        FieldType::Uuid => Some("UUID"),
        FieldType::Timestamp => Some("ISO 8601 timestamp"),
        FieldType::Json => Some("arbitrary JSON"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Directive, EnumDef, FieldDef, FieldType, ModelDef, RequestDef};

    #[test]
    fn generates_interface() {
        let model = ModelDef {
            name: "Product".into(),
            fields: vec![FieldDef {
                name: "price".into(),
                ty: FieldType::Float,
                directives: vec![],
                optional: false,
            }],
        };
        let src = generate_interface(&model);
        assert!(src.contains("export interface Product"), "missing interface");
        assert!(src.contains("price: number"), "missing price field");
    }

    #[test]
    fn generates_enum_type() {
        let enum_def = EnumDef {
            name: "UserRole".into(),
            variants: vec!["USER".into(), "ADMIN".into(), "SUPER_ADMIN".into()],
        };
        let src = generate_enum_type(&enum_def);
        assert!(src.contains("export type UserRole ="));
        assert!(src.contains("\"USER\""));
        assert!(src.contains("\"ADMIN\""));
        assert!(src.contains("\"SUPER_ADMIN\""));
    }

    #[test]
    fn generates_request_interface() {
        let request = RequestDef {
            name: "CreateUserRequest".into(),
            fields: vec![FieldDef {
                name: "email".into(),
                ty: FieldType::String,
                directives: vec![Directive::Email],
                optional: false,
            }],
        };
        let src = generate_request_interface(&request);
        assert!(src.contains("export interface CreateUserRequest"));
        assert!(src.contains("email: string"));
    }

    #[test]
    fn uuid_field_has_comment() {
        let model = ModelDef {
            name: "User".into(),
            fields: vec![FieldDef {
                name: "id".into(),
                ty: FieldType::Uuid,
                directives: vec![],
                optional: false,
            }],
        };
        let src = generate_interface(&model);
        assert!(src.contains("// UUID"), "missing UUID comment");
    }

    #[test]
    fn optional_field_uses_question_mark() {
        let model = ModelDef {
            name: "User".into(),
            fields: vec![FieldDef {
                name: "bio".into(),
                ty: FieldType::String,
                directives: vec![],
                optional: true,
            }],
        };
        let src = generate_interface(&model);
        assert!(src.contains("bio?: string"));
    }

    #[test]
    fn array_field_type() {
        let model = ModelDef {
            name: "Post".into(),
            fields: vec![FieldDef {
                name: "tags".into(),
                ty: FieldType::Array(Box::new(FieldType::String)),
                directives: vec![],
                optional: false,
            }],
        };
        let src = generate_interface(&model);
        assert!(src.contains("tags: string[]"));
    }

    #[test]
    fn model_reference_type() {
        let model = ModelDef {
            name: "Post".into(),
            fields: vec![FieldDef {
                name: "author".into(),
                ty: FieldType::Model("User".into()),
                directives: vec![],
                optional: false,
            }],
        };
        let src = generate_interface(&model);
        assert!(src.contains("author: User"));
    }
}
