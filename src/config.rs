use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct ProjectConfig {
    #[serde(default)]
    pub type_map: BTreeMap<String, String>,
    #[serde(default = "default_comment_prefix")]
    pub comment_prefix: String,
    #[serde(default)]
    pub comment_suffix: String,
    #[serde(default)]
    pub hooks: Hooks,
}

#[derive(Debug, Default, Deserialize)]
pub struct Hooks {
    /// Command to run on each created file. `{file}` is replaced with the file path.
    pub post_generate: Option<String>,
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
            type_map: BTreeMap::new(),
            comment_prefix: default_comment_prefix(),
            comment_suffix: String::new(),
            hooks: Hooks::default(),
        });
    }
    let content = std::fs::read_to_string(&config_path)
        .with_context(|| format!("failed to read {}", config_path.display()))?;
    let config: ProjectConfig = toml::from_str(&content)
        .with_context(|| format!("failed to parse {}", config_path.display()))?;
    Ok(config)
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
        assert_eq!(config.comment_prefix, "//");
    }

    #[test]
    fn load_config_missing_file_returns_default() {
        let dir = TempDir::new().unwrap();
        let config = load_config(dir.path()).unwrap();
        assert!(config.type_map.is_empty());
        assert_eq!(config.comment_prefix, "//");
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

        assert!(
            config_path.exists(),
            "config file not written at {}",
            config_path.display()
        );
        let config = load_config(dir.path()).unwrap();
        assert_eq!(config.comment_prefix, "<!--");
        assert_eq!(config.comment_suffix, "-->");
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
        assert_eq!(
            config.hooks.post_generate.as_deref(),
            Some("rustfmt {file}")
        );
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
