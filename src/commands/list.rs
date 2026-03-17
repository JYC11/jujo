use crate::discovery;
use crate::generator;
use anyhow::Result;
use serde::Serialize;
use std::path::Path;

#[derive(Serialize)]
struct GeneratorSummary {
    name: String,
    description: String,
}

/// Run `jujo list`, showing all available generators.
pub fn run(jujo_root: &Path, json: bool) -> Result<()> {
    let generators = find_all_generators(jujo_root)?;

    if json {
        let summaries: Vec<GeneratorSummary> = generators
            .iter()
            .map(|(_, def)| GeneratorSummary {
                name: def.generator.name.clone(),
                description: def.generator.description.clone(),
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&summaries)?);
    } else if generators.is_empty() {
        println!("No generators found in .jujo/templates/");
        println!("Create one with: mkdir -p .jujo/templates/<name> && edit .jujo/templates/<name>/generator.toml");
    } else {
        for (dir_name, def) in &generators {
            println!("  {} — {}", dir_name, def.generator.description);
        }
    }

    Ok(())
}

/// Scan .jujo/templates/ for all valid generators.
pub fn find_all_generators(jujo_root: &Path) -> Result<Vec<(String, generator::GeneratorDef)>> {
    let templates_dir = jujo_root.join(".jujo/templates");
    if !templates_dir.is_dir() {
        return Ok(vec![]);
    }

    let mut generators = Vec::new();
    let mut entries: Vec<_> = std::fs::read_dir(&templates_dir)?
        .filter_map(|e| e.ok())
        .collect();
    entries.sort_by_key(|e| e.file_name());

    for entry in entries {
        if !entry.path().is_dir() {
            continue;
        }
        let dir_name = entry.file_name().to_string_lossy().to_string();
        let gen_dir = entry.path();
        if gen_dir.join("generator.toml").exists() {
            match generator::load_generator(&gen_dir) {
                Ok(def) => generators.push((dir_name, def)),
                Err(_) => continue, // Skip invalid generators in list mode.
            }
        }
    }

    Ok(generators)
}
