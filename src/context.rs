use crate::config::ProjectConfig;
use crate::fields;
use crate::generator::{GeneratorDef, InputType};
use anyhow::{Result, bail};
use std::collections::BTreeMap;
use tera::Context;

/// Parse `--var key=value` strings into a multi-map. Splits on first `=` only.
/// Repeated keys accumulate values (supports both comma-separated and repeated --var).
pub fn parse_vars(vars: &[String]) -> Result<BTreeMap<String, Vec<String>>> {
    let mut map: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for var in vars {
        let (key, value) = var.split_once('=').ok_or_else(|| {
            anyhow::anyhow!("invalid --var format: \"{var}\". Expected key=value")
        })?;
        map.entry(key.to_string())
            .or_default()
            .push(value.to_string());
    }
    Ok(map)
}

/// Build a Tera context from a generator definition and user-provided variables.
pub fn build_context(
    def: &GeneratorDef,
    vars: &BTreeMap<String, Vec<String>>,
    config: &ProjectConfig,
) -> Result<Context> {
    let mut ctx = Context::new();

    for input in &def.inputs {
        let values = vars.get(&input.name);

        match input.r#type {
            InputType::String => {
                // Take the last value if multiple provided.
                let value = values.and_then(|v| v.last().map(|s| s.as_str()));
                if let Some(value) = value {
                    ctx.insert(&input.name, value);
                } else if let Some(default) = &input.default {
                    let value = default.as_str().ok_or_else(|| {
                        anyhow::anyhow!("default for \"{}\" must be a string", input.name)
                    })?;
                    ctx.insert(&input.name, value);
                } else if input.required {
                    bail!(
                        "missing required input \"{}\". Provide it with --var {}=<value>",
                        input.name,
                        input.name
                    );
                }
            }
            InputType::FieldArray => {
                if let Some(values) = values {
                    let field_specs = fields::parse_field_list(values, &config.type_map)?;
                    ctx.insert(&input.name, &field_specs);
                } else if input.required {
                    bail!(
                        "missing required input \"{}\". Provide it with --var {}=<field:type>",
                        input.name,
                        input.name
                    );
                } else {
                    // Insert empty array for optional field arrays.
                    ctx.insert(&input.name, &Vec::<fields::FieldSpec>::new());
                }
            }
            InputType::StringArray => {
                if let Some(values) = values {
                    // Flatten comma-separated values.
                    let flat: Vec<String> = values
                        .iter()
                        .flat_map(|v| v.split(',').map(|s| s.trim().to_string()))
                        .filter(|s| !s.is_empty())
                        .collect();
                    ctx.insert(&input.name, &flat);
                } else if input.required {
                    bail!(
                        "missing required input \"{}\". Provide it with --var {}=<value>",
                        input.name,
                        input.name
                    );
                } else {
                    ctx.insert(&input.name, &Vec::<String>::new());
                }
            }
            InputType::Bool => {
                let value = values.and_then(|v| v.last().map(|s| s.as_str()));
                if let Some(value) = value {
                    let b = match value {
                        "true" | "1" | "yes" => true,
                        "false" | "0" | "no" => false,
                        _ => bail!(
                            "invalid bool value \"{}\" for input \"{}\". Use true/false",
                            value,
                            input.name
                        ),
                    };
                    ctx.insert(&input.name, &b);
                } else if let Some(default) = &input.default {
                    let b = default.as_bool().ok_or_else(|| {
                        anyhow::anyhow!("default for \"{}\" must be a boolean", input.name)
                    })?;
                    ctx.insert(&input.name, &b);
                } else if input.required {
                    bail!("missing required input \"{}\"", input.name);
                }
            }
            InputType::Int => {
                let value = values.and_then(|v| v.last().map(|s| s.as_str()));
                if let Some(value) = value {
                    let n: i64 = value.parse().map_err(|_| {
                        anyhow::anyhow!(
                            "invalid integer \"{}\" for input \"{}\"",
                            value,
                            input.name
                        )
                    })?;
                    ctx.insert(&input.name, &n);
                } else if let Some(default) = &input.default {
                    let n = default.as_integer().ok_or_else(|| {
                        anyhow::anyhow!("default for \"{}\" must be an integer", input.name)
                    })?;
                    ctx.insert(&input.name, &n);
                } else if input.required {
                    bail!("missing required input \"{}\"", input.name);
                }
            }
        }
    }

    Ok(ctx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config;
    use crate::generator::{GeneratorMeta, InputDef};

    fn make_def(inputs: Vec<InputDef>) -> GeneratorDef {
        GeneratorDef {
            generator: GeneratorMeta {
                name: "test".into(),
                description: "test".into(),
            },
            inputs,
            actions: vec![],
        }
    }

    fn string_input(name: &str, required: bool, default: Option<&str>) -> InputDef {
        InputDef {
            name: name.into(),
            r#type: InputType::String,
            description: String::new(),
            required,
            default: default.map(|s| toml::Value::String(s.into())),
        }
    }

    fn field_input(name: &str, required: bool) -> InputDef {
        InputDef {
            name: name.into(),
            r#type: InputType::FieldArray,
            description: String::new(),
            required,
            default: None,
        }
    }

    fn empty_config() -> ProjectConfig {
        ProjectConfig {
            type_map: BTreeMap::new(),
            comment_prefix: "//".into(),
            comment_suffix: String::new(),
            hooks: config::Hooks::default(),
        }
    }

    fn rust_config() -> ProjectConfig {
        ProjectConfig {
            type_map: BTreeMap::from([
                ("string".into(), "String".into()),
                ("int".into(), "i64".into()),
                ("bool".into(), "bool".into()),
                ("decimal".into(), "rust_decimal::Decimal".into()),
            ]),
            comment_prefix: "//".into(),
            comment_suffix: String::new(),
            hooks: config::Hooks::default(),
        }
    }

    #[test]
    fn parse_vars_basic() {
        let vars = vec!["name=hello".into(), "count=42".into()];
        let map = parse_vars(&vars).unwrap();
        assert_eq!(map["name"], vec!["hello"]);
        assert_eq!(map["count"], vec!["42"]);
    }

    #[test]
    fn parse_vars_value_with_equals() {
        let vars = vec!["expr=a=b+c".into()];
        let map = parse_vars(&vars).unwrap();
        assert_eq!(map["expr"], vec!["a=b+c"]);
    }

    #[test]
    fn parse_vars_repeated_key() {
        let vars = vec!["fields=a:string".into(), "fields=b:int".into()];
        let map = parse_vars(&vars).unwrap();
        assert_eq!(map["fields"], vec!["a:string", "b:int"]);
    }

    #[test]
    fn parse_vars_missing_equals() {
        let vars = vec!["noequals".into()];
        let err = parse_vars(&vars).unwrap_err();
        assert!(err.to_string().contains("Expected key=value"));
    }

    #[test]
    fn build_context_required_present() {
        let def = make_def(vec![string_input("name", true, None)]);
        let vars = BTreeMap::from([("name".into(), vec!["hello".into()])]);
        let ctx = build_context(&def, &vars, &empty_config()).unwrap();
        assert_eq!(ctx.get("name").unwrap(), "hello");
    }

    #[test]
    fn build_context_required_missing() {
        let def = make_def(vec![string_input("name", true, None)]);
        let vars = BTreeMap::new();
        let err = build_context(&def, &vars, &empty_config()).unwrap_err();
        assert!(err.to_string().contains("missing required input"));
        assert!(err.to_string().contains("--var name="));
    }

    #[test]
    fn build_context_default_applied() {
        let def = make_def(vec![string_input("greeting", false, Some("hello"))]);
        let vars = BTreeMap::new();
        let ctx = build_context(&def, &vars, &empty_config()).unwrap();
        assert_eq!(ctx.get("greeting").unwrap(), "hello");
    }

    #[test]
    fn build_context_default_overridden() {
        let def = make_def(vec![string_input("greeting", false, Some("hello"))]);
        let vars = BTreeMap::from([("greeting".into(), vec!["hi".into()])]);
        let ctx = build_context(&def, &vars, &empty_config()).unwrap();
        assert_eq!(ctx.get("greeting").unwrap(), "hi");
    }

    #[test]
    fn build_context_field_array() {
        let def = make_def(vec![field_input("fields", true)]);
        let vars = BTreeMap::from([("fields".into(), vec!["title:string,price:decimal?".into()])]);
        let ctx = build_context(&def, &vars, &rust_config()).unwrap();
        let fields = ctx.get("fields").unwrap();
        let arr = fields.as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0].get("name").unwrap(), "title");
        assert_eq!(arr[0].get("mapped_type").unwrap(), "String");
        assert_eq!(arr[1].get("name").unwrap(), "price");
        assert_eq!(arr[1].get("nullable").unwrap(), true);
    }

    #[test]
    fn build_context_field_array_empty_type_map() {
        let def = make_def(vec![field_input("fields", true)]);
        let vars = BTreeMap::from([("fields".into(), vec!["title:string".into()])]);
        let err = build_context(&def, &vars, &empty_config()).unwrap_err();
        assert!(err.to_string().contains("no mapping in config.toml"));
    }

    #[test]
    fn build_context_bool_input() {
        let def = make_def(vec![InputDef {
            name: "active".into(),
            r#type: InputType::Bool,
            description: String::new(),
            required: true,
            default: None,
        }]);
        let vars = BTreeMap::from([("active".into(), vec!["true".into()])]);
        let ctx = build_context(&def, &vars, &empty_config()).unwrap();
        assert_eq!(ctx.get("active").unwrap(), true);

        let vars_false = BTreeMap::from([("active".into(), vec!["false".into()])]);
        let ctx2 = build_context(&def, &vars_false, &empty_config()).unwrap();
        assert_eq!(ctx2.get("active").unwrap(), false);
    }

    #[test]
    fn build_context_int_input() {
        let def = make_def(vec![InputDef {
            name: "count".into(),
            r#type: InputType::Int,
            description: String::new(),
            required: true,
            default: None,
        }]);
        let vars = BTreeMap::from([("count".into(), vec!["42".into()])]);
        let ctx = build_context(&def, &vars, &empty_config()).unwrap();
        assert_eq!(ctx.get("count").unwrap(), 42);
    }

    #[test]
    fn build_context_string_array_input() {
        let def = make_def(vec![InputDef {
            name: "tags".into(),
            r#type: InputType::StringArray,
            description: String::new(),
            required: true,
            default: None,
        }]);
        // Comma-separated.
        let vars = BTreeMap::from([("tags".into(), vec!["alpha,beta,gamma".into()])]);
        let ctx = build_context(&def, &vars, &empty_config()).unwrap();
        let tags = ctx.get("tags").unwrap().as_array().unwrap();
        assert_eq!(tags.len(), 3);
        assert_eq!(tags[0], "alpha");
        assert_eq!(tags[2], "gamma");
    }
}
