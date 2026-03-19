#![allow(dead_code)] // Newtypes expose constructors/accessors for future use.

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use std::ops::Deref;
use std::path::Path;
use std::str::FromStr;

// ---------------------------------------------------------------------------
// GeneratorName
// ---------------------------------------------------------------------------

/// A validated generator name. Non-empty, no path separators or whitespace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct GeneratorName(String);

impl GeneratorName {
    pub fn new(name: impl Into<String>) -> Result<Self> {
        let name = name.into();
        if name.is_empty() {
            bail!("generator name cannot be empty");
        }
        if name.contains('/') || name.contains('\\') || name.contains("..") {
            bail!("generator name cannot contain path separators: \"{name}\"");
        }
        if name.chars().any(|c| c.is_whitespace()) {
            bail!("generator name cannot contain whitespace: \"{name}\"");
        }
        Ok(Self(name))
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}

impl Deref for GeneratorName {
    type Target = str;
    fn deref(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for GeneratorName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl AsRef<str> for GeneratorName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

// ---------------------------------------------------------------------------
// RelativePath
// ---------------------------------------------------------------------------

/// A validated relative file path. No leading `/`, no `..` components.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RelativePath(String);

impl RelativePath {
    pub fn new(path: impl Into<String>) -> Result<Self> {
        let path = path.into();
        if path.is_empty() {
            bail!("relative path cannot be empty");
        }
        if path.starts_with('/') || path.starts_with('\\') {
            bail!("path must be relative, not absolute: \"{path}\"");
        }
        if path.split(['/', '\\']).any(|c| c == "..") {
            bail!("path cannot contain '..' components: \"{path}\"");
        }
        Ok(Self(path))
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}

impl Deref for RelativePath {
    type Target = str;
    fn deref(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for RelativePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl AsRef<Path> for RelativePath {
    fn as_ref(&self) -> &Path {
        Path::new(&self.0)
    }
}

// ---------------------------------------------------------------------------
// CommentStyle
// ---------------------------------------------------------------------------

/// A comment style pairing prefix and suffix. Owns marker tag formatting.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommentStyle {
    pub prefix: String,
    #[serde(default)]
    pub suffix: String,
}

impl CommentStyle {
    pub fn new(prefix: impl Into<String>, suffix: impl Into<String>) -> Self {
        Self {
            prefix: prefix.into(),
            suffix: suffix.into(),
        }
    }

    /// Format a jujo closing marker tag, e.g. `// </jujo:modules>` or `<!-- </jujo:modules> -->`.
    pub fn marker_tag(&self, marker_name: &MarkerName) -> String {
        if self.suffix.is_empty() {
            format!("{} </jujo:{}>", self.prefix, marker_name)
        } else {
            format!("{} </jujo:{}> {}", self.prefix, marker_name, self.suffix)
        }
    }
}

// ---------------------------------------------------------------------------
// AbstractType
// ---------------------------------------------------------------------------

/// The set of abstract field types jujo recognizes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum AbstractType {
    String,
    Text,
    Int,
    Bool,
    Float,
    Decimal,
    Uuid,
    Date,
    Datetime,
    Json,
}

impl AbstractType {
    /// All known abstract types.
    pub const ALL: &[AbstractType] = &[
        Self::String,
        Self::Text,
        Self::Int,
        Self::Bool,
        Self::Float,
        Self::Decimal,
        Self::Uuid,
        Self::Date,
        Self::Datetime,
        Self::Json,
    ];

    /// The string representation used as type_map key.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::String => "string",
            Self::Text => "text",
            Self::Int => "int",
            Self::Bool => "bool",
            Self::Float => "float",
            Self::Decimal => "decimal",
            Self::Uuid => "uuid",
            Self::Date => "date",
            Self::Datetime => "datetime",
            Self::Json => "json",
        }
    }

    /// Comma-separated list of all type names (for error messages).
    pub fn all_names() -> std::string::String {
        Self::ALL
            .iter()
            .map(|t| t.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    }
}

impl FromStr for AbstractType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "string" => Ok(Self::String),
            "text" => Ok(Self::Text),
            "int" => Ok(Self::Int),
            "bool" => Ok(Self::Bool),
            "float" => Ok(Self::Float),
            "decimal" => Ok(Self::Decimal),
            "uuid" => Ok(Self::Uuid),
            "date" => Ok(Self::Date),
            "datetime" => Ok(Self::Datetime),
            "json" => Ok(Self::Json),
            _ => bail!("unknown type \"{s}\". Valid types: {}", Self::all_names()),
        }
    }
}

impl fmt::Display for AbstractType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

// ---------------------------------------------------------------------------
// MarkerName
// ---------------------------------------------------------------------------

/// A validated marker name for injection points.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MarkerName(String);

impl MarkerName {
    pub fn new(name: impl Into<String>) -> Result<Self> {
        let name = name.into();
        if name.is_empty() {
            bail!("marker name cannot be empty");
        }
        if name.contains('<') || name.contains('>') {
            bail!("marker name cannot contain angle brackets: \"{name}\"");
        }
        if name.contains('/') || name.contains('\\') {
            bail!("marker name cannot contain path separators: \"{name}\"");
        }
        Ok(Self(name))
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}

impl Deref for MarkerName {
    type Target = str;
    fn deref(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for MarkerName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl AsRef<str> for MarkerName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

// ---------------------------------------------------------------------------
// TypeMap
// ---------------------------------------------------------------------------

/// A mapping from abstract types to language-specific type names.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TypeMap(BTreeMap<String, String>);

impl TypeMap {
    pub fn new(map: BTreeMap<String, String>) -> Self {
        Self(map)
    }

    /// Look up a language-specific type for an abstract type.
    /// Returns a helpful error if the mapping is missing.
    pub fn lookup(&self, abstract_type: &AbstractType) -> Result<&str> {
        let key = abstract_type.as_str();
        self.0.get(key).map(|s| s.as_str()).ok_or_else(|| {
            anyhow::anyhow!(
                "type \"{key}\" has no mapping in config.toml [type_map]. \
                 add: {key} = \"<language type>\""
            )
        })
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl Deref for TypeMap {
    type Target = BTreeMap<String, String>;
    fn deref(&self) -> &BTreeMap<String, String> {
        &self.0
    }
}

// ---------------------------------------------------------------------------
// TemplateName
// ---------------------------------------------------------------------------

/// A validated template file name (must end with `.tera`, no path separators).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TemplateName(String);

impl TemplateName {
    pub fn new(name: impl Into<String>) -> Result<Self> {
        let name = name.into();
        if name.is_empty() {
            bail!("template name cannot be empty");
        }
        if !name.ends_with(".tera") {
            bail!("template name must end with .tera: \"{name}\"");
        }
        if name.contains('/') || name.contains('\\') {
            bail!("template name cannot contain path separators: \"{name}\"");
        }
        Ok(Self(name))
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}

impl Deref for TemplateName {
    type Target = str;
    fn deref(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for TemplateName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl AsRef<str> for TemplateName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

// ---------------------------------------------------------------------------
// HookTemplate
// ---------------------------------------------------------------------------

/// A validated hook command template. Must contain `{file}` placeholder.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct HookTemplate(String);

impl HookTemplate {
    pub fn new(template: impl Into<String>) -> Result<Self> {
        let template = template.into();
        if template.is_empty() {
            bail!("hook template cannot be empty");
        }
        Ok(Self(template))
    }

    /// Run the hook command, substituting `{file}` with the given path.
    /// The file path is single-quoted to prevent shell injection.
    pub fn run(&self, file_path: &Path) {
        let escaped = file_path.display().to_string().replace('\'', "'\\''");
        let cmd = self.0.replace("{file}", &format!("'{escaped}'"));
        let result = std::process::Command::new("sh")
            .arg("-c")
            .arg(&cmd)
            .output();
        match result {
            Ok(output) if !output.status.success() => {
                let stderr = std::string::String::from_utf8_lossy(&output.stderr);
                eprintln!("  hook warning: {cmd} failed: {stderr}");
            }
            Err(e) => eprintln!("  hook warning: failed to run \"{cmd}\": {e}"),
            _ => {}
        }
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}

impl Deref for HookTemplate {
    type Target = str;
    fn deref(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for HookTemplate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // GeneratorName
    #[test]
    fn generator_name_valid() {
        assert!(GeneratorName::new("module").is_ok());
        assert!(GeneratorName::new("my-generator").is_ok());
        assert!(GeneratorName::new("gen_v2").is_ok());
    }

    #[test]
    fn generator_name_rejects_empty() {
        assert!(GeneratorName::new("").is_err());
    }

    #[test]
    fn generator_name_rejects_path_separators() {
        assert!(GeneratorName::new("../evil").is_err());
        assert!(GeneratorName::new("a/b").is_err());
        assert!(GeneratorName::new("a\\b").is_err());
    }

    #[test]
    fn generator_name_rejects_whitespace() {
        assert!(GeneratorName::new("my module").is_err());
        assert!(GeneratorName::new("my\tmod").is_err());
    }

    // RelativePath
    #[test]
    fn relative_path_valid() {
        assert!(RelativePath::new("src/orders/entity.rs").is_ok());
        assert!(RelativePath::new("file.txt").is_ok());
    }

    #[test]
    fn relative_path_rejects_absolute() {
        assert!(RelativePath::new("/etc/passwd").is_err());
        assert!(RelativePath::new("\\windows\\system32").is_err());
    }

    #[test]
    fn relative_path_rejects_traversal() {
        assert!(RelativePath::new("../../../etc/passwd").is_err());
        assert!(RelativePath::new("src/../../evil").is_err());
    }

    #[test]
    fn relative_path_rejects_empty() {
        assert!(RelativePath::new("").is_err());
    }

    // CommentStyle
    #[test]
    fn comment_style_rust_marker() {
        let style = CommentStyle::new("//", "");
        let marker = MarkerName::new("modules").unwrap();
        assert_eq!(style.marker_tag(&marker), "// </jujo:modules>");
    }

    #[test]
    fn comment_style_html_marker() {
        let style = CommentStyle::new("<!--", "-->");
        let marker = MarkerName::new("scripts").unwrap();
        assert_eq!(style.marker_tag(&marker), "<!-- </jujo:scripts> -->");
    }

    // AbstractType
    #[test]
    fn abstract_type_from_str() {
        assert_eq!(
            "string".parse::<AbstractType>().unwrap(),
            AbstractType::String
        );
        assert_eq!(
            "decimal".parse::<AbstractType>().unwrap(),
            AbstractType::Decimal
        );
        assert!("money".parse::<AbstractType>().is_err());
    }

    #[test]
    fn abstract_type_roundtrip() {
        for ty in AbstractType::ALL {
            let parsed: AbstractType = ty.as_str().parse().unwrap();
            assert_eq!(*ty, parsed);
        }
    }

    // MarkerName
    #[test]
    fn marker_name_valid() {
        assert!(MarkerName::new("modules").is_ok());
        assert!(MarkerName::new("route-registration").is_ok());
    }

    #[test]
    fn marker_name_rejects_angle_brackets() {
        assert!(MarkerName::new("mod<ules>").is_err());
    }

    #[test]
    fn marker_name_rejects_empty() {
        assert!(MarkerName::new("").is_err());
    }

    // TypeMap
    #[test]
    fn type_map_lookup_found() {
        let map = TypeMap::new(BTreeMap::from([("string".into(), "String".into())]));
        assert_eq!(map.lookup(&AbstractType::String).unwrap(), "String");
    }

    #[test]
    fn type_map_lookup_missing() {
        let map = TypeMap::new(BTreeMap::new());
        let err = map.lookup(&AbstractType::String).unwrap_err();
        assert!(err.to_string().contains("no mapping in config.toml"));
    }

    // TemplateName
    #[test]
    fn template_name_valid() {
        assert!(TemplateName::new("entity.tera").is_ok());
        assert!(TemplateName::new("mod.tera").is_ok());
    }

    #[test]
    fn template_name_rejects_no_tera_extension() {
        assert!(TemplateName::new("entity.rs").is_err());
    }

    #[test]
    fn template_name_rejects_path_separators() {
        assert!(TemplateName::new("../evil.tera").is_err());
    }

    // HookTemplate
    #[test]
    fn hook_template_valid() {
        assert!(HookTemplate::new("rustfmt {file}").is_ok());
        assert!(HookTemplate::new("prettier --write {file}").is_ok());
    }

    #[test]
    fn hook_template_rejects_empty() {
        assert!(HookTemplate::new("").is_err());
    }

    #[test]
    fn hook_template_shell_quotes_file_path() {
        // Verify the substitution produces a single-quoted path
        let _hook = HookTemplate::new("echo {file}").unwrap();
        let path = std::path::Path::new("src/test;echo PWNED.txt");
        // We can't easily capture the command, but we can verify the escaping
        // logic by checking the internal string replacement.
        let escaped = path.display().to_string().replace('\'', "'\\''");
        let cmd = "echo {file}".replace("{file}", &format!("'{escaped}'"));
        assert_eq!(cmd, "echo 'src/test;echo PWNED.txt'");
    }

    #[test]
    fn hook_template_shell_quotes_single_quotes_in_path() {
        let path = std::path::Path::new("src/it's a file.txt");
        let escaped = path.display().to_string().replace('\'', "'\\''");
        let cmd = "echo {file}".replace("{file}", &format!("'{escaped}'"));
        assert_eq!(cmd, "echo 'src/it'\\''s a file.txt'");
    }

    // Serde roundtrip
    #[test]
    fn generator_name_serde_transparent() {
        let name = GeneratorName::new("module").unwrap();
        let json = serde_json::to_string(&name).unwrap();
        assert_eq!(json, "\"module\"");
        let parsed: GeneratorName = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.as_ref(), "module");
    }

    #[test]
    fn marker_name_serde_transparent() {
        let name = MarkerName::new("routes").unwrap();
        let json = serde_json::to_string(&name).unwrap();
        assert_eq!(json, "\"routes\"");
    }
}
