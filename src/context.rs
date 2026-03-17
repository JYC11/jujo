use crate::generator::{GeneratorDef, InputType};
use anyhow::{bail, Result};
use std::collections::BTreeMap;
use tera::Context;

/// Parse `--var key=value` strings into a map. Splits on first `=` only.
pub fn parse_vars(vars: &[String]) -> Result<BTreeMap<String, String>> {
    let mut map = BTreeMap::new();
    for var in vars {
        let (key, value) = var
            .split_once('=')
            .ok_or_else(|| anyhow::anyhow!("invalid --var format: \"{var}\". Expected key=value"))?;
        map.insert(key.to_string(), value.to_string());
    }
    Ok(map)
}

/// Build a Tera context from a generator definition and user-provided variables.
pub fn build_context(
    def: &GeneratorDef,
    vars: &BTreeMap<String, String>,
) -> Result<Context> {
    let mut ctx = Context::new();

    for input in &def.inputs {
        match input.r#type {
            InputType::String => {
                if let Some(value) = vars.get(&input.name) {
                    ctx.insert(&input.name, value);
                } else if let Some(default) = &input.default {
                    let value = default
                        .as_str()
                        .ok_or_else(|| anyhow::anyhow!("default for \"{}\" must be a string", input.name))?;
                    ctx.insert(&input.name, value);
                } else if input.required {
                    bail!(
                        "missing required input \"{}\". Provide it with --var {}=<value>",
                        input.name,
                        input.name
                    );
                }
            }
            // Phase 2+: other types handled later.
            _ => {
                bail!(
                    "input type {:?} for \"{}\" is not yet supported",
                    input.r#type,
                    input.name
                );
            }
        }
    }

    Ok(ctx)
}

#[cfg(test)]
mod tests {
    use super::*;
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

    #[test]
    fn parse_vars_basic() {
        let vars = vec!["name=hello".into(), "count=42".into()];
        let map = parse_vars(&vars).unwrap();
        assert_eq!(map["name"], "hello");
        assert_eq!(map["count"], "42");
    }

    #[test]
    fn parse_vars_value_with_equals() {
        let vars = vec!["expr=a=b+c".into()];
        let map = parse_vars(&vars).unwrap();
        assert_eq!(map["expr"], "a=b+c");
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
        let vars = BTreeMap::from([("name".into(), "hello".into())]);
        let ctx = build_context(&def, &vars).unwrap();
        assert_eq!(ctx.get("name").unwrap(), "hello");
    }

    #[test]
    fn build_context_required_missing() {
        let def = make_def(vec![string_input("name", true, None)]);
        let vars = BTreeMap::new();
        let err = build_context(&def, &vars).unwrap_err();
        assert!(err.to_string().contains("missing required input"));
        assert!(err.to_string().contains("--var name="));
    }

    #[test]
    fn build_context_default_applied() {
        let def = make_def(vec![string_input("greeting", false, Some("hello"))]);
        let vars = BTreeMap::new();
        let ctx = build_context(&def, &vars).unwrap();
        assert_eq!(ctx.get("greeting").unwrap(), "hello");
    }

    #[test]
    fn build_context_default_overridden() {
        let def = make_def(vec![string_input("greeting", false, Some("hello"))]);
        let vars = BTreeMap::from([("greeting".into(), "hi".into())]);
        let ctx = build_context(&def, &vars).unwrap();
        assert_eq!(ctx.get("greeting").unwrap(), "hi");
    }
}
