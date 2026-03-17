use anyhow::{bail, Result};
use std::path::Path;

struct LanguagePreset {
    name: &'static str,
    comment_prefix: &'static str,
    comment_suffix: &'static str,
    type_map_toml: &'static str,
}

const LANGUAGES: &[LanguagePreset] = &[
    LanguagePreset {
        name: "rust",
        comment_prefix: "//",
        comment_suffix: "",
        type_map_toml: r#"string = "String"
text = "String"
int = "i64"
bool = "bool"
float = "f64"
decimal = "rust_decimal::Decimal"
uuid = "String"
date = "chrono::NaiveDate"
datetime = "chrono::DateTime<Utc>"
json = "serde_json::Value""#,
    },
    LanguagePreset {
        name: "go",
        comment_prefix: "//",
        comment_suffix: "",
        type_map_toml: r#"string = "string"
text = "string"
int = "int64"
bool = "bool"
float = "float64"
decimal = "decimal.Decimal"
uuid = "string"
date = "time.Time"
datetime = "time.Time"
json = "json.RawMessage""#,
    },
    LanguagePreset {
        name: "python",
        comment_prefix: "#",
        comment_suffix: "",
        type_map_toml: r#"string = "str"
text = "str"
int = "int"
bool = "bool"
float = "float"
decimal = "Decimal"
uuid = "str"
date = "date"
datetime = "datetime"
json = "dict""#,
    },
    LanguagePreset {
        name: "typescript",
        comment_prefix: "//",
        comment_suffix: "",
        type_map_toml: r#"string = "string"
text = "string"
int = "number"
bool = "boolean"
float = "number"
decimal = "number"
uuid = "string"
date = "Date"
datetime = "Date"
json = "Record<string, unknown>""#,
    },
    LanguagePreset {
        name: "java",
        comment_prefix: "//",
        comment_suffix: "",
        type_map_toml: r#"string = "String"
text = "String"
int = "Long"
bool = "Boolean"
float = "Double"
decimal = "BigDecimal"
uuid = "UUID"
date = "LocalDate"
datetime = "Instant"
json = "JsonNode""#,
    },
];

fn language_names() -> Vec<&'static str> {
    LANGUAGES.iter().map(|l| l.name).collect()
}

fn find_language(name: &str) -> Option<&'static LanguagePreset> {
    LANGUAGES.iter().find(|l| l.name == name)
}

/// Run `jujo init`, creating .jujo/ with config and example generator.
pub fn run(lang: Option<&str>) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let jujo_dir = cwd.join(".jujo");

    if jujo_dir.exists() {
        bail!(".jujo/ already exists in {}. Remove it first to re-initialize.", cwd.display());
    }

    let lang_name = match lang {
        Some(l) => {
            if find_language(l).is_none() {
                bail!(
                    "unknown language \"{l}\". Available: {}",
                    language_names().join(", ")
                );
            }
            l.to_string()
        }
        None => {
            let options = language_names();
            inquire::Select::new("Select language:", options.clone())
                .prompt()
                .map(|s| s.to_string())?
        }
    };

    let preset = find_language(&lang_name).unwrap();

    let templates_dir = jujo_dir.join("templates/example");
    std::fs::create_dir_all(&templates_dir)?;

    let config_content = format!(
        "comment_prefix = \"{}\"\ncomment_suffix = \"{}\"\n\n[type_map]\n{}\n",
        preset.comment_prefix, preset.comment_suffix, preset.type_map_toml
    );
    std::fs::write(jujo_dir.join("config.toml"), config_content)?;

    write_example_generator(&templates_dir)?;

    println!("Initialized .jujo/ with {lang_name} type map.");
    println!("  .jujo/config.toml");
    println!("  .jujo/templates/example/generator.toml");
    println!("\nTry: jujo generate example --var name=hello");

    Ok(())
}

fn write_example_generator(dir: &Path) -> Result<()> {
    std::fs::write(
        dir.join("generator.toml"),
        r#"[generator]
name = "example"
description = "A simple example generator"

[[inputs]]
name = "name"
type = "string"
description = "A name to greet"
required = true

[[actions]]
type = "create"
template = "greeting.tera"
output = "{{ name }}.txt"
"#,
    )?;

    std::fs::write(
        dir.join("greeting.tera"),
        "Hello, {{ name | pascal_case }}! Welcome to jujo.\n",
    )?;

    Ok(())
}
