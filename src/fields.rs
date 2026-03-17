use crate::types::{AbstractType, TypeMap};
use anyhow::{Result, bail};
use serde::Serialize;

/// A parsed field specification.
#[derive(Debug, Clone, Serialize)]
pub struct FieldSpec {
    pub name: String,
    pub r#type: AbstractType,
    pub mapped_type: String,
    pub nullable: bool,
}

/// Parse a single field spec like "title:string" or "price:decimal?".
pub fn parse_field(spec: &str, type_map: &TypeMap) -> Result<FieldSpec> {
    let (name, type_part) = spec.split_once(':').ok_or_else(|| {
        anyhow::anyhow!("invalid field spec \"{spec}\". Expected name:type (e.g. \"title:string\")")
    })?;

    if name.is_empty() {
        bail!("field name cannot be empty in \"{spec}\"");
    }

    let nullable = type_part.ends_with('?');
    let type_str = if nullable {
        &type_part[..type_part.len() - 1]
    } else {
        type_part
    };

    let abstract_type: AbstractType = type_str.parse().map_err(|_| {
        anyhow::anyhow!(
            "unknown type \"{type_str}\" in field \"{spec}\". Valid types: {}",
            AbstractType::all_names()
        )
    })?;

    let mapped_type = type_map.lookup(&abstract_type)?;

    Ok(FieldSpec {
        name: name.to_string(),
        r#type: abstract_type,
        mapped_type: mapped_type.to_string(),
        nullable,
    })
}

/// Parse a comma-separated string of field specs OR merge multiple values.
pub fn parse_field_list(values: &[String], type_map: &TypeMap) -> Result<Vec<FieldSpec>> {
    let mut fields = Vec::new();
    for value in values {
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
    use std::collections::BTreeMap;

    fn rust_type_map() -> TypeMap {
        TypeMap::new(BTreeMap::from([
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
        ]))
    }

    #[test]
    fn parse_simple_field() {
        let map = rust_type_map();
        let field = parse_field("title:string", &map).unwrap();
        assert_eq!(field.name, "title");
        assert_eq!(field.r#type, AbstractType::String);
        assert_eq!(field.mapped_type, "String");
        assert!(!field.nullable);
    }

    #[test]
    fn parse_nullable_field() {
        let map = rust_type_map();
        let field = parse_field("price:decimal?", &map).unwrap();
        assert_eq!(field.name, "price");
        assert_eq!(field.r#type, AbstractType::Decimal);
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
        let empty_map = TypeMap::default();
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
