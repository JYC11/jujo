use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LanguagePreset {
    comment_prefix: String,
    #[serde(default)]
    comment_suffix: String,
    type_map: BTreeMap<String, String>,
}

/// All built-in language presets. Sorted alphabetically.
fn builtin_presets() -> BTreeMap<String, LanguagePreset> {
    let mut m = BTreeMap::new();

    m.insert(
        "css".into(),
        LanguagePreset {
            comment_prefix: "/*".into(),
            comment_suffix: "*/".into(),
            type_map: type_map(&[
                ("string", "string"),
                ("text", "string"),
                ("int", "number"),
                ("bool", "boolean"),
                ("float", "number"),
                ("decimal", "number"),
                ("uuid", "string"),
                ("date", "string"),
                ("datetime", "string"),
                ("json", "string"),
            ]),
        },
    );

    m.insert(
        "csharp".into(),
        LanguagePreset {
            comment_prefix: "//".into(),
            comment_suffix: String::new(),
            type_map: type_map(&[
                ("string", "string"),
                ("text", "string"),
                ("int", "long"),
                ("bool", "bool"),
                ("float", "double"),
                ("decimal", "decimal"),
                ("uuid", "Guid"),
                ("date", "DateOnly"),
                ("datetime", "DateTimeOffset"),
                ("json", "JsonElement"),
            ]),
        },
    );

    m.insert(
        "elixir".into(),
        LanguagePreset {
            comment_prefix: "#".into(),
            comment_suffix: String::new(),
            type_map: type_map(&[
                ("string", "String.t()"),
                ("text", "String.t()"),
                ("int", "integer()"),
                ("bool", "boolean()"),
                ("float", "float()"),
                ("decimal", "Decimal.t()"),
                ("uuid", "String.t()"),
                ("date", "Date.t()"),
                ("datetime", "DateTime.t()"),
                ("json", "map()"),
            ]),
        },
    );

    m.insert(
        "html".into(),
        LanguagePreset {
            comment_prefix: "<!--".into(),
            comment_suffix: "-->".into(),
            type_map: type_map(&[
                ("string", "string"),
                ("text", "string"),
                ("int", "number"),
                ("bool", "boolean"),
                ("float", "number"),
                ("decimal", "number"),
                ("uuid", "string"),
                ("date", "string"),
                ("datetime", "string"),
                ("json", "string"),
            ]),
        },
    );

    m.insert(
        "go".into(),
        LanguagePreset {
            comment_prefix: "//".into(),
            comment_suffix: String::new(),
            type_map: type_map(&[
                ("string", "string"),
                ("text", "string"),
                ("int", "int64"),
                ("bool", "bool"),
                ("float", "float64"),
                ("decimal", "decimal.Decimal"),
                ("uuid", "string"),
                ("date", "time.Time"),
                ("datetime", "time.Time"),
                ("json", "json.RawMessage"),
            ]),
        },
    );

    m.insert(
        "java".into(),
        LanguagePreset {
            comment_prefix: "//".into(),
            comment_suffix: String::new(),
            type_map: type_map(&[
                ("string", "String"),
                ("text", "String"),
                ("int", "Long"),
                ("bool", "Boolean"),
                ("float", "Double"),
                ("decimal", "BigDecimal"),
                ("uuid", "UUID"),
                ("date", "LocalDate"),
                ("datetime", "Instant"),
                ("json", "JsonNode"),
            ]),
        },
    );

    m.insert(
        "kotlin".into(),
        LanguagePreset {
            comment_prefix: "//".into(),
            comment_suffix: String::new(),
            type_map: type_map(&[
                ("string", "String"),
                ("text", "String"),
                ("int", "Long"),
                ("bool", "Boolean"),
                ("float", "Double"),
                ("decimal", "BigDecimal"),
                ("uuid", "UUID"),
                ("date", "LocalDate"),
                ("datetime", "Instant"),
                ("json", "JsonElement"),
            ]),
        },
    );

    m.insert(
        "php".into(),
        LanguagePreset {
            comment_prefix: "//".into(),
            comment_suffix: String::new(),
            type_map: type_map(&[
                ("string", "string"),
                ("text", "string"),
                ("int", "int"),
                ("bool", "bool"),
                ("float", "float"),
                ("decimal", "string"),
                ("uuid", "string"),
                ("date", "DateTimeImmutable"),
                ("datetime", "DateTimeImmutable"),
                ("json", "array"),
            ]),
        },
    );

    m.insert(
        "python".into(),
        LanguagePreset {
            comment_prefix: "#".into(),
            comment_suffix: String::new(),
            type_map: type_map(&[
                ("string", "str"),
                ("text", "str"),
                ("int", "int"),
                ("bool", "bool"),
                ("float", "float"),
                ("decimal", "Decimal"),
                ("uuid", "str"),
                ("date", "date"),
                ("datetime", "datetime"),
                ("json", "dict"),
            ]),
        },
    );

    m.insert(
        "ruby".into(),
        LanguagePreset {
            comment_prefix: "#".into(),
            comment_suffix: String::new(),
            type_map: type_map(&[
                ("string", "String"),
                ("text", "String"),
                ("int", "Integer"),
                ("bool", "Boolean"),
                ("float", "Float"),
                ("decimal", "BigDecimal"),
                ("uuid", "String"),
                ("date", "Date"),
                ("datetime", "DateTime"),
                ("json", "Hash"),
            ]),
        },
    );

    m.insert(
        "rust".into(),
        LanguagePreset {
            comment_prefix: "//".into(),
            comment_suffix: String::new(),
            type_map: type_map(&[
                ("string", "String"),
                ("text", "String"),
                ("int", "i64"),
                ("bool", "bool"),
                ("float", "f64"),
                ("decimal", "rust_decimal::Decimal"),
                ("uuid", "String"),
                ("date", "chrono::NaiveDate"),
                ("datetime", "chrono::DateTime<Utc>"),
                ("json", "serde_json::Value"),
            ]),
        },
    );

    m.insert(
        "swift".into(),
        LanguagePreset {
            comment_prefix: "//".into(),
            comment_suffix: String::new(),
            type_map: type_map(&[
                ("string", "String"),
                ("text", "String"),
                ("int", "Int64"),
                ("bool", "Bool"),
                ("float", "Double"),
                ("decimal", "Decimal"),
                ("uuid", "UUID"),
                ("date", "Date"),
                ("datetime", "Date"),
                ("json", "Any"),
            ]),
        },
    );

    m.insert(
        "typescript".into(),
        LanguagePreset {
            comment_prefix: "//".into(),
            comment_suffix: String::new(),
            type_map: type_map(&[
                ("string", "string"),
                ("text", "string"),
                ("int", "number"),
                ("bool", "boolean"),
                ("float", "number"),
                ("decimal", "number"),
                ("uuid", "string"),
                ("date", "Date"),
                ("datetime", "Date"),
                ("json", "Record<string, unknown>"),
            ]),
        },
    );

    m
}

fn type_map(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| ((*k).into(), (*v).into()))
        .collect()
}

/// Load user-defined presets from ~/.jujo/languages.toml, merging with builtins.
/// User presets override builtins for the same language name.
fn load_presets() -> BTreeMap<String, LanguagePreset> {
    let mut presets = builtin_presets();

    if let Some(home) = dirs::home_dir() {
        let global_file = home.join(".jujo").join("languages.toml");
        if let Ok(contents) = std::fs::read_to_string(&global_file) {
            if let Ok(user_presets) = toml::from_str::<BTreeMap<String, LanguagePreset>>(&contents)
            {
                for (name, preset) in user_presets {
                    presets.insert(name, preset);
                }
            }
        }
    }

    presets
}

/// Serialize presets to TOML for the reference file.
fn presets_to_toml(presets: &BTreeMap<String, LanguagePreset>) -> String {
    let mut out = String::from(
        "# Language presets for jujo.\n\
         # To add custom languages, copy this file to ~/.jujo/languages.toml and edit.\n\
         # User-defined presets override built-in defaults.\n\n",
    );

    for (name, preset) in presets {
        out.push_str(&format!("[{name}]\n"));
        out.push_str(&format!("comment_prefix = {:?}\n", preset.comment_prefix));
        if !preset.comment_suffix.is_empty() {
            out.push_str(&format!("comment_suffix = {:?}\n", preset.comment_suffix));
        }
        out.push_str("\n");
        out.push_str(&format!("[{name}.type_map]\n"));
        for (k, v) in &preset.type_map {
            out.push_str(&format!("{k} = {v:?}\n"));
        }
        out.push('\n');
    }

    out
}

/// Format a preset's type_map as TOML key-value lines (for config.toml).
fn type_map_toml(preset: &LanguagePreset) -> String {
    preset
        .type_map
        .iter()
        .map(|(k, v)| format!("{k} = {v:?}"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Run `jujo init`, creating .jujo/ with config and example generator.
pub fn run(lang: Option<&str>) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let jujo_dir = cwd.join(".jujo");

    if jujo_dir.exists() {
        bail!(
            ".jujo/ already exists in {}. Remove it first to re-initialize.",
            cwd.display()
        );
    }

    let presets = load_presets();
    let names: Vec<&str> = presets.keys().map(|s| s.as_str()).collect();

    let lang_name = match lang {
        Some(l) => {
            if !presets.contains_key(l) {
                bail!("unknown language \"{l}\". Available: {}", names.join(", "));
            }
            l.to_string()
        }
        None => {
            let options: Vec<String> = names.iter().map(|s| (*s).to_string()).collect();
            inquire::Select::new("Select language:", options).prompt()?
        }
    };

    let preset = &presets[&lang_name];

    let templates_dir = jujo_dir.join("templates/example");
    std::fs::create_dir_all(&templates_dir)?;

    let config_content = format!(
        "comment_prefix = {:?}\ncomment_suffix = {:?}\n\n[type_map]\n{}\n",
        preset.comment_prefix,
        preset.comment_suffix,
        type_map_toml(preset)
    );
    std::fs::write(jujo_dir.join("config.toml"), config_content)?;

    std::fs::write(jujo_dir.join("languages.toml"), presets_to_toml(&presets))?;

    write_example_generator(&templates_dir)?;

    println!("Initialized .jujo/ with {lang_name} type map.");
    println!("  .jujo/config.toml");
    println!("  .jujo/languages.toml  (all language presets — copy to ~/.jujo/ to customize)");
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_presets_contain_all_languages() {
        let presets = builtin_presets();
        let expected = [
            "css",
            "csharp",
            "elixir",
            "go",
            "html",
            "java",
            "kotlin",
            "php",
            "python",
            "ruby",
            "rust",
            "swift",
            "typescript",
        ];
        for lang in expected {
            assert!(presets.contains_key(lang), "missing builtin preset: {lang}");
        }
    }

    #[test]
    fn all_presets_have_complete_type_maps() {
        let required_types = [
            "string", "text", "int", "bool", "float", "decimal", "uuid", "date", "datetime", "json",
        ];
        for (name, preset) in builtin_presets() {
            for ty in required_types {
                assert!(
                    preset.type_map.contains_key(ty),
                    "language {name} missing type mapping for {ty}"
                );
            }
        }
    }

    #[test]
    fn presets_to_toml_roundtrips() {
        let presets = builtin_presets();
        let toml_str = presets_to_toml(&presets);
        let parsed: BTreeMap<String, LanguagePreset> =
            toml::from_str(&toml_str).expect("generated TOML should parse");
        assert_eq!(presets.len(), parsed.len());
        for (name, original) in &presets {
            let parsed_preset = parsed
                .get(name)
                .unwrap_or_else(|| panic!("parsed TOML missing language: {name}"));
            assert_eq!(original.comment_prefix, parsed_preset.comment_prefix);
            assert_eq!(original.type_map, parsed_preset.type_map);
        }
    }

    #[test]
    fn type_map_toml_formats_correctly() {
        let preset = &builtin_presets()["rust"];
        let toml_str = type_map_toml(preset);
        assert!(toml_str.contains(r#"string = "String""#));
        assert!(toml_str.contains(r#"int = "i64""#));
    }

    #[test]
    fn user_preset_overrides_builtin() {
        // Simulate: user file has a custom "rust" with different int type
        let toml_str = r#"
[rust]
comment_prefix = "//"
[rust.type_map]
string = "String"
text = "String"
int = "i32"
bool = "bool"
float = "f64"
decimal = "rust_decimal::Decimal"
uuid = "String"
date = "chrono::NaiveDate"
datetime = "chrono::DateTime<Utc>"
json = "serde_json::Value"
"#;
        let user_presets: BTreeMap<String, LanguagePreset> = toml::from_str(toml_str).unwrap();
        let mut presets = builtin_presets();
        for (name, preset) in user_presets {
            presets.insert(name, preset);
        }
        assert_eq!(presets["rust"].type_map["int"], "i32");
    }

    #[test]
    fn user_preset_adds_new_language() {
        let toml_str = r#"
[zig]
comment_prefix = "//"
[zig.type_map]
string = "[]const u8"
text = "[]const u8"
int = "i64"
bool = "bool"
float = "f64"
decimal = "f128"
uuid = "[]const u8"
date = "i64"
datetime = "i64"
json = "std.json.Value"
"#;
        let user_presets: BTreeMap<String, LanguagePreset> = toml::from_str(toml_str).unwrap();
        let mut presets = builtin_presets();
        for (name, preset) in user_presets {
            presets.insert(name, preset);
        }
        assert!(presets.contains_key("zig"));
        assert_eq!(presets["zig"].type_map["string"], "[]const u8");
    }
}
