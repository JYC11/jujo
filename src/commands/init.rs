use anyhow::{bail, Result};
use std::path::Path;

/// Bundled language presets: (name, comment_prefix, comment_suffix, type_map_toml).
const LANGUAGES: &[(&str, &str, &str, &str)] = &[
    (
        "rust",
        "//",
        "",
        r#"string = "String"
text = "String"
int = "i64"
bool = "bool"
float = "f64"
decimal = "rust_decimal::Decimal"
uuid = "String"
date = "chrono::NaiveDate"
datetime = "chrono::DateTime<Utc>"
json = "serde_json::Value""#,
    ),
    (
        "go",
        "//",
        "",
        r#"string = "string"
text = "string"
int = "int64"
bool = "bool"
float = "float64"
decimal = "decimal.Decimal"
uuid = "string"
date = "time.Time"
datetime = "time.Time"
json = "json.RawMessage""#,
    ),
    (
        "python",
        "#",
        "",
        r#"string = "str"
text = "str"
int = "int"
bool = "bool"
float = "float"
decimal = "Decimal"
uuid = "str"
date = "date"
datetime = "datetime"
json = "dict""#,
    ),
    (
        "typescript",
        "//",
        "",
        r#"string = "string"
text = "string"
int = "number"
bool = "boolean"
float = "number"
decimal = "number"
uuid = "string"
date = "Date"
datetime = "Date"
json = "Record<string, unknown>""#,
    ),
    (
        "java",
        "//",
        "",
        r#"string = "String"
text = "String"
int = "Long"
bool = "Boolean"
float = "Double"
decimal = "BigDecimal"
uuid = "UUID"
date = "LocalDate"
datetime = "Instant"
json = "JsonNode""#,
    ),
];

fn language_names() -> Vec<&'static str> {
    LANGUAGES.iter().map(|(name, _, _, _)| *name).collect()
}

fn find_language(name: &str) -> Option<&'static (&'static str, &'static str, &'static str, &'static str)> {
    LANGUAGES.iter().find(|(n, _, _, _)| *n == name)
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
            // Interactive selection.
            let options = language_names();
            inquire::Select::new("Select language:", options.clone())
                .prompt()
                .map(|s| s.to_string())?
        }
    };

    let (_, prefix, suffix, type_map) = find_language(&lang_name).unwrap();

    // Create directory structure.
    let templates_dir = jujo_dir.join("templates/example");
    std::fs::create_dir_all(&templates_dir)?;

    // Write config.toml.
    let config_content = format!(
        "comment_prefix = \"{prefix}\"\ncomment_suffix = \"{suffix}\"\n\n[type_map]\n{type_map}\n"
    );
    std::fs::write(jujo_dir.join("config.toml"), config_content)?;

    // Write example generator.
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
