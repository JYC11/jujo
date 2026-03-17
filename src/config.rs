use crate::types::{CommentStyle, HookTemplate, TypeMap};
use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;

/// Project configuration loaded from `.jujo/config.toml`.
#[derive(Debug)]
pub struct ProjectConfig {
    pub type_map: TypeMap,
    pub comment_style: CommentStyle,
    pub hooks: Hooks,
}

#[derive(Debug, Default)]
pub struct Hooks {
    pub post_generate: Option<HookTemplate>,
}

/// Raw TOML structure — deserialized then converted to domain types.
#[derive(Deserialize)]
struct RawConfig {
    #[serde(default)]
    type_map: BTreeMap<String, String>,
    #[serde(default = "default_comment_prefix")]
    comment_prefix: String,
    #[serde(default)]
    comment_suffix: String,
    #[serde(default)]
    hooks: RawHooks,
}

#[derive(Default, Deserialize)]
struct RawHooks {
    post_generate: Option<String>,
}

fn default_comment_prefix() -> String {
    "//".into()
}

/// Load project config from `.jujo/config.toml`.
/// Returns a default config if the file doesn't exist.
pub fn load_config(jujo_root: &Path) -> Result<ProjectConfig> {
    let config_path = jujo_root.join(".jujo/config.toml");
    if !config_path.exists() {
        return Ok(ProjectConfig {
            type_map: TypeMap::default(),
            comment_style: CommentStyle::new("//", ""),
            hooks: Hooks::default(),
        });
    }
    let content = std::fs::read_to_string(&config_path)
        .with_context(|| format!("failed to read {}", config_path.display()))?;
    let raw: RawConfig = toml::from_str(&content)
        .with_context(|| format!("failed to parse {}", config_path.display()))?;

    let hook = match raw.hooks.post_generate {
        Some(h) => Some(HookTemplate::new(h)?),
        None => None,
    };

    Ok(ProjectConfig {
        type_map: TypeMap::new(raw.type_map),
        comment_style: CommentStyle::new(raw.comment_prefix, raw.comment_suffix),
        hooks: Hooks {
            post_generate: hook,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn load_config_with_type_map() {
        let dir = TempDir::new().unwrap();
        std::fs::create_dir_all(dir.path().join(".jujo")).unwrap();
        std::fs::write(
            dir.path().join(".jujo/config.toml"),
            "comment_prefix = \"//\"\ncomment_suffix = \"\"\n\n[type_map]\nstring = \"String\"\nint = \"i64\"\nbool = \"bool\"\ndecimal = \"rust_decimal::Decimal\"\n",
        )
        .unwrap();

        let config = load_config(dir.path()).unwrap();
        assert_eq!(config.type_map["string"], "String");
        assert_eq!(config.type_map["int"], "i64");
        assert_eq!(config.type_map["decimal"], "rust_decimal::Decimal");
        assert_eq!(config.comment_style.prefix, "//");
    }

    #[test]
    fn load_config_missing_file_returns_default() {
        let dir = TempDir::new().unwrap();
        let config = load_config(dir.path()).unwrap();
        assert!(config.type_map.is_empty());
        assert_eq!(config.comment_style.prefix, "//");
    }

    #[test]
    fn load_config_html_comments() {
        let dir = TempDir::new().unwrap();
        std::fs::create_dir_all(dir.path().join(".jujo")).unwrap();
        let config_path = dir.path().join(".jujo/config.toml");
        std::fs::write(
            &config_path,
            "comment_prefix = \"<!--\"\ncomment_suffix = \"-->\"\n\n[type_map]\nstring = \"string\"\n",
        )
        .unwrap();

        let config = load_config(dir.path()).unwrap();
        assert_eq!(config.comment_style.prefix, "<!--");
        assert_eq!(config.comment_style.suffix, "-->");
    }

    #[test]
    fn load_config_with_hooks() {
        let dir = TempDir::new().unwrap();
        std::fs::create_dir_all(dir.path().join(".jujo")).unwrap();
        std::fs::write(
            dir.path().join(".jujo/config.toml"),
            "comment_prefix = \"//\"\n\n[hooks]\npost_generate = \"rustfmt {file}\"\n\n[type_map]\nstring = \"String\"\n",
        )
        .unwrap();

        let config = load_config(dir.path()).unwrap();
        assert!(config.hooks.post_generate.is_some());
    }

    #[test]
    fn load_config_no_hooks_defaults_to_none() {
        let dir = TempDir::new().unwrap();
        std::fs::create_dir_all(dir.path().join(".jujo")).unwrap();
        std::fs::write(
            dir.path().join(".jujo/config.toml"),
            "comment_prefix = \"//\"\n\n[type_map]\nstring = \"String\"\n",
        )
        .unwrap();

        let config = load_config(dir.path()).unwrap();
        assert!(config.hooks.post_generate.is_none());
    }
}
