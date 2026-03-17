use crate::commands::list;
use crate::generator::{self, Action};
use crate::render;
use anyhow::Result;
use std::path::Path;

/// Run `jujo validate`, checking all generators and templates.
pub fn run(jujo_root: &Path) -> Result<()> {
    let templates_dir = jujo_root.join(".jujo/templates");
    if !templates_dir.is_dir() {
        println!("No .jujo/templates/ directory found. Nothing to validate.");
        return Ok(());
    }

    let mut errors: Vec<String> = Vec::new();
    let mut checked = 0;

    let entries: Vec<_> = std::fs::read_dir(&templates_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .collect();

    for entry in &entries {
        let dir_name = entry.file_name().to_string_lossy().to_string();
        let gen_dir = entry.path();
        let toml_path = gen_dir.join("generator.toml");

        if !toml_path.exists() {
            continue;
        }

        checked += 1;

        // Check 1: generator.toml parses correctly.
        let def = match generator::load_generator(&gen_dir) {
            Ok(d) => d,
            Err(e) => {
                errors.push(format!("{dir_name}: {e}"));
                continue;
            }
        };

        // Check 2: all template files referenced in create actions exist.
        for action in &def.actions {
            if let Action::Create { template, .. } = action {
                let template_path = gen_dir.join(template);
                if !template_path.exists() {
                    errors.push(format!(
                        "{dir_name}: template file \"{template}\" not found (referenced in create action)"
                    ));
                }
            }
        }

        // Check 3: templates parse without syntax errors.
        match render::create_tera(&gen_dir) {
            Ok(_) => {}
            Err(e) => {
                errors.push(format!("{dir_name}: {e}"));
            }
        }
    }

    if checked == 0 {
        println!("No generators found in .jujo/templates/");
        return Ok(());
    }

    if errors.is_empty() {
        println!("Validated {checked} generator(s). All OK.");
    } else {
        println!("Validated {checked} generator(s). {} error(s):\n", errors.len());
        for error in &errors {
            println!("  {error}");
        }
        anyhow::bail!("validation failed with {} error(s)", errors.len());
    }

    Ok(())
}
