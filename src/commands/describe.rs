use crate::discovery;
use crate::generator::{self, Action, InputType};
use anyhow::Result;
use serde::Serialize;
use std::path::Path;

#[derive(Serialize)]
struct GeneratorSchema {
    name: String,
    description: String,
    inputs: Vec<InputSchema>,
    actions: Vec<ActionSchema>,
}

#[derive(Serialize)]
struct InputSchema {
    name: String,
    r#type: String,
    description: String,
    required: bool,
    default: Option<serde_json::Value>,
}

#[derive(Serialize)]
struct ActionSchema {
    r#type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    template: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    output: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    target: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    marker: Option<String>,
}

/// Run `jujo describe <name>`, showing the generator's full schema.
pub fn run(jujo_root: &Path, name: &str, json: bool) -> Result<()> {
    let gen_dir = discovery::generator_dir(jujo_root, name)?;
    let def = generator::load_generator(&gen_dir)?;

    let schema = build_schema(&def);

    if json {
        println!("{}", serde_json::to_string_pretty(&schema)?);
    } else {
        println!("Generator: {}", schema.name);
        println!("  {}", schema.description);
        println!();
        println!("Inputs:");
        for input in &schema.inputs {
            let req = if input.required {
                "required"
            } else {
                "optional"
            };
            let default = input
                .default
                .as_ref()
                .map(|d| format!(" (default: {d})"))
                .unwrap_or_default();
            println!(
                "  {} ({}, {}{}) — {}",
                input.name, input.r#type, req, default, input.description
            );
        }
        println!();
        println!("Actions:");
        for action in &schema.actions {
            match action.r#type.as_str() {
                "create" => {
                    println!(
                        "  create {} → {}",
                        action.template.as_deref().unwrap_or("?"),
                        action.output.as_deref().unwrap_or("?")
                    );
                }
                "inject" => {
                    println!(
                        "  inject → {} (marker: {})",
                        action.target.as_deref().unwrap_or("?"),
                        action.marker.as_deref().unwrap_or("?")
                    );
                }
                _ => {}
            }
        }
    }

    Ok(())
}

fn build_schema(def: &generator::GeneratorDef) -> GeneratorSchema {
    let inputs = def
        .inputs
        .iter()
        .map(|i| {
            let type_str = match i.r#type {
                InputType::String => "string",
                InputType::StringArray => "string[]",
                InputType::Bool => "bool",
                InputType::Int => "int",
                InputType::FieldArray => "field[]",
            };
            let default = i.default.as_ref().map(|v| match v {
                toml::Value::String(s) => serde_json::Value::String(s.clone()),
                toml::Value::Boolean(b) => serde_json::Value::Bool(*b),
                toml::Value::Integer(n) => serde_json::json!(*n),
                _ => serde_json::Value::String(v.to_string()),
            });
            InputSchema {
                name: i.name.clone(),
                r#type: type_str.to_string(),
                description: i.description.clone(),
                required: i.required,
                default,
            }
        })
        .collect();

    let actions = def
        .actions
        .iter()
        .map(|a| match a {
            Action::Create { template, output } => ActionSchema {
                r#type: "create".into(),
                template: Some(template.clone()),
                output: Some(output.clone()),
                target: None,
                marker: None,
            },
            Action::Inject { target, marker, .. } => ActionSchema {
                r#type: "inject".into(),
                template: None,
                output: None,
                target: Some(target.clone()),
                marker: Some(marker.clone()),
            },
        })
        .collect();

    GeneratorSchema {
        name: def.generator.name.clone(),
        description: def.generator.description.clone(),
        inputs,
        actions,
    }
}
