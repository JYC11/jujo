use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct GeneratorDef {
    pub generator: GeneratorMeta,
    #[serde(default)]
    pub inputs: Vec<InputDef>,
    #[serde(default)]
    pub actions: Vec<Action>,
}

#[derive(Debug, Deserialize)]
pub struct GeneratorMeta {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Deserialize)]
pub struct InputDef {
    pub name: String,
    pub r#type: InputType,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_true")]
    pub required: bool,
    pub default: Option<toml::Value>,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum InputType {
    String,
    #[serde(alias = "string[]")]
    StringArray,
    Bool,
    Int,
    #[serde(alias = "field[]")]
    FieldArray,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Action {
    Create {
        template: String,
        output: String,
    },
    Inject {
        target: String,
        marker: String,
        content: String,
    },
}

/// Load and parse a generator.toml from a directory.
pub fn load_generator(gen_dir: &Path) -> Result<GeneratorDef> {
    let toml_path = gen_dir.join("generator.toml");
    let content = std::fs::read_to_string(&toml_path)
        .with_context(|| format!("failed to read {}", toml_path.display()))?;
    let def: GeneratorDef = toml::from_str(&content)
        .with_context(|| format!("failed to parse {}", toml_path.display()))?;
    Ok(def)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn parse_minimal_generator() {
        let toml = r#"
[generator]
name = "example"
description = "An example generator"

[[inputs]]
name = "module_name"
type = "string"
description = "The module name"
required = true

[[actions]]
type = "create"
template = "hello.tera"
output = "src/{{ module_name }}.rs"
"#;
        let def: GeneratorDef = toml::from_str(toml).unwrap();
        assert_eq!(def.generator.name, "example");
        assert_eq!(def.inputs.len(), 1);
        assert_eq!(def.inputs[0].name, "module_name");
        assert_eq!(def.inputs[0].r#type, InputType::String);
        assert!(def.inputs[0].required);
        assert_eq!(def.actions.len(), 1);
        match &def.actions[0] {
            Action::Create { template, output } => {
                assert_eq!(template, "hello.tera");
                assert_eq!(output, "src/{{ module_name }}.rs");
            }
            _ => panic!("expected Create action"),
        }
    }

    #[test]
    fn parse_malformed_toml() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("generator.toml"), "this is not [valid toml").unwrap();
        let err = load_generator(dir.path()).unwrap_err();
        assert!(err.to_string().contains("failed to parse"));
    }

    #[test]
    fn parse_wrong_schema() {
        let dir = TempDir::new().unwrap();
        // Valid TOML but missing [generator] section.
        std::fs::write(
            dir.path().join("generator.toml"),
            "[wrong]\nkey = \"value\"",
        )
        .unwrap();
        let err = load_generator(dir.path()).unwrap_err();
        assert!(err.to_string().contains("failed to parse"));
    }

    #[test]
    fn parse_with_default_value() {
        let toml = r#"
[generator]
name = "test"
description = "test"

[[inputs]]
name = "greeting"
type = "string"
required = false
default = "hello"
"#;
        let def: GeneratorDef = toml::from_str(toml).unwrap();
        assert!(!def.inputs[0].required);
        assert_eq!(
            def.inputs[0].default.as_ref().unwrap().as_str().unwrap(),
            "hello"
        );
    }

    #[test]
    fn parse_inject_action() {
        let toml = r#"
[generator]
name = "test"
description = "test"

[[actions]]
type = "inject"
target = "src/main.rs"
marker = "modules"
content = "mod {{ module_name }};"
"#;
        let def: GeneratorDef = toml::from_str(toml).unwrap();
        match &def.actions[0] {
            Action::Inject {
                target,
                marker,
                content,
            } => {
                assert_eq!(target, "src/main.rs");
                assert_eq!(marker, "modules");
                assert_eq!(content, "mod {{ module_name }};");
            }
            _ => panic!("expected Inject action"),
        }
    }
}
