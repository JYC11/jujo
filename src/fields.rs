use anyhow::{Result, bail};
use serde::Serialize;
use std::collections::BTreeMap;

/// Known abstract types that jujo recognizes.
const KNOWN_TYPES: &[&str] = &[
    "string", "text", "int", "bool", "float", "decimal", "uuid", "date", "datetime", "json",
];

/// A parsed field specification.
#[derive(Debug, Clone, Serialize)]
pub struct FieldSpec {
    pub name: String,
    pub r#type: String,
    pub mapped_type: String,
    pub nullable: bool,
}

/// Parse a single field spec like "title:string" or "price:decimal?".
pub fn parse_field(spec: &str, type_map: &BTreeMap<String, String>) -> Result<FieldSpec> {
    let (name, type_part) = spec.split_once(':').ok_or_else(|| {
        anyhow::anyhow!("invalid field spec \"{spec}\". Expected name:type (e.g. \"title:string\")")
    })?;

    if name.is_empty() {
        bail!("field name cannot be empty in \"{spec}\"");
    }

    let nullable = type_part.ends_with('?');
    let abstract_type = if nullable {
        &type_part[..type_part.len() - 1]
    } else {
        type_part
    };

    if !KNOWN_TYPES.contains(&abstract_type) {
        bail!(
            "unknown type \"{abstract_type}\" in field \"{spec}\". Valid types: {}",
            KNOWN_TYPES.join(", ")
        );
    }

    let mapped_type = type_map.get(abstract_type).ok_or_else(|| {
        anyhow::anyhow!(
            "type \"{abstract_type}\" has no mapping in config.toml [type_map]. \
             add: {abstract_type} = \"<language type>\""
        )
    })?;

    Ok(FieldSpec {
        name: name.to_string(),
        r#type: abstract_type.to_string(),
        mapped_type: mapped_type.clone(),
        nullable,
    })
}

/// Parse a comma-separated string of field specs OR merge multiple values.
/// Supports both `--var fields="a:string,b:int"` and repeated `--var fields=a:string`.
pub fn parse_field_list(
    values: &[String],
    type_map: &BTreeMap<String, String>,
) -> Result<Vec<FieldSpec>> {
    let mut fields = Vec::new();
    for value in values {
        // Split on comma for comma-separated values.
        for spec in value.split(',') {
            let spec = spec.trim();
            if !spec.is_empty() {
                fields.push(parse_field(spec, type_map)?);
            }
        }
    }
    Ok(fields)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rust_type_map() -> BTreeMap<String, String> {
        BTreeMap::from([
            ("string".into(), "String".into()),
            ("text".into(), "String".into()),
            ("int".into(), "i64".into()),
            ("bool".into(), "bool".into()),
            ("float".into(), "f64".into()),
            ("decimal".into(), "rust_decimal::Decimal".into()),
            ("uuid".into(), "String".into()),
            ("date".into(), "chrono::NaiveDate".into()),
            ("datetime".into(), "chrono::DateTime<Utc>".into()),
            ("json".into(), "serde_json::Value".into()),
        ])
    }

    #[test]
    fn parse_simple_field() {
        let map = rust_type_map();
        let field = parse_field("title:string", &map).unwrap();
        assert_eq!(field.name, "title");
        assert_eq!(field.r#type, "string");
        assert_eq!(field.mapped_type, "String");
        assert!(!field.nullable);
    }

    #[test]
    fn parse_nullable_field() {
        let map = rust_type_map();
        let field = parse_field("price:decimal?", &map).unwrap();
        assert_eq!(field.name, "price");
        assert_eq!(field.r#type, "decimal");
        assert_eq!(field.mapped_type, "rust_decimal::Decimal");
        assert!(field.nullable);
    }

    #[test]
    fn parse_unknown_type() {
        let map = rust_type_map();
        let err = parse_field("amount:money", &map).unwrap_err();
        assert!(err.to_string().contains("unknown type \"money\""));
        assert!(err.to_string().contains("Valid types:"));
    }

    #[test]
    fn parse_malformed_no_colon() {
        let map = rust_type_map();
        let err = parse_field("title", &map).unwrap_err();
        assert!(err.to_string().contains("Expected name:type"));
    }

    #[test]
    fn parse_empty_name() {
        let map = rust_type_map();
        let err = parse_field(":string", &map).unwrap_err();
        assert!(err.to_string().contains("field name cannot be empty"));
    }

    #[test]
    fn parse_unmapped_type() {
        let empty_map = BTreeMap::new();
        let err = parse_field("title:string", &empty_map).unwrap_err();
        assert!(err.to_string().contains("no mapping in config.toml"));
    }

    #[test]
    fn parse_comma_separated_list() {
        let map = rust_type_map();
        let values = vec!["title:string,price:decimal?,active:bool".into()];
        let fields = parse_field_list(&values, &map).unwrap();
        assert_eq!(fields.len(), 3);
        assert_eq!(fields[0].name, "title");
        assert_eq!(fields[1].name, "price");
        assert!(fields[1].nullable);
        assert_eq!(fields[2].name, "active");
    }

    #[test]
    fn parse_repeated_var_list() {
        let map = rust_type_map();
        let values = vec!["title:string".into(), "price:decimal?".into()];
        let fields = parse_field_list(&values, &map).unwrap();
        assert_eq!(fields.len(), 2);
        assert_eq!(fields[0].name, "title");
        assert_eq!(fields[1].name, "price");
    }

    #[test]
    fn parse_mixed_comma_and_repeated() {
        let map = rust_type_map();
        let values = vec!["title:string,price:decimal?".into(), "active:bool".into()];
        let fields = parse_field_list(&values, &map).unwrap();
        assert_eq!(fields.len(), 3);
    }
}
