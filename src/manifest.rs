use crate::types::{GeneratorName, MarkerName, RelativePath};
use anyhow::{Context, Result};
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, Serialize)]
pub struct GenerationResult {
    pub generator: GeneratorName,
    pub timestamp: String,
    pub inputs: BTreeMap<String, serde_json::Value>,
    pub created: Vec<ManifestCreatedFile>,
    pub injected: Vec<ManifestInjectedContent>,
    pub customize: Vec<ManifestCustomizeMarker>,
}

#[derive(Debug, Serialize)]
pub struct ManifestCreatedFile {
    pub path: RelativePath,
    pub template: String,
}

#[derive(Debug, Serialize)]
pub struct ManifestInjectedContent {
    pub path: RelativePath,
    pub marker: MarkerName,
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct ManifestCustomizeMarker {
    pub path: RelativePath,
    pub line: usize,
    pub hint: String,
}

/// Write the generation manifest to `.jujo/last-generate.json`.
pub fn write_manifest(manifest_path: &Path, result: &GenerationResult) -> Result<()> {
    let json =
        serde_json::to_string_pretty(result).context("failed to serialize generation manifest")?;
    std::fs::write(manifest_path, json)
        .with_context(|| format!("failed to write manifest to {}", manifest_path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::GeneratorName;
    use tempfile::TempDir;

    #[test]
    fn write_and_read_manifest() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("last-generate.json");

        let result = GenerationResult {
            generator: GeneratorName::new("module").unwrap(),
            timestamp: "2026-03-17T14:30:00Z".into(),
            inputs: BTreeMap::from([(
                "module_name".into(),
                serde_json::Value::String("orders".into()),
            )]),
            created: vec![ManifestCreatedFile {
                path: RelativePath::new("src/orders/mod.rs").unwrap(),
                template: "mod.rs.tera".into(),
            }],
            injected: vec![],
            customize: vec![],
        };

        write_manifest(&path, &result).unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();

        assert_eq!(parsed["generator"], "module");
        assert_eq!(parsed["created"][0]["path"], "src/orders/mod.rs");
        assert_eq!(parsed["inputs"]["module_name"], "orders");
    }
}
