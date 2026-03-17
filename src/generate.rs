use crate::types::RelativePath;
use crate::{config, context, discovery, file_ops, generator, manifest, markers, render};
use anyhow::Result;
use owo_colors::OwoColorize;
use std::collections::BTreeMap;

pub fn run(
    name: &str,
    vars: &[String],
    force: bool,
    skip_existing: bool,
    dry_run: bool,
    json_output: bool,
) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let root = discovery::find_jujo_root(&cwd)?;
    let gen_dir = discovery::generator_dir(&root, name)?;
    let project_config = config::load_config(&root)?;

    let def = generator::load_generator(&gen_dir)?;
    let var_map = context::parse_vars(vars)?;
    let ctx = context::build_context(&def, &var_map, &project_config)?;
    let tera = render::create_tera(&gen_dir)?;

    let conflict_mode = if skip_existing {
        file_ops::ConflictMode::Skip
    } else {
        file_ops::ConflictMode::Error
    };

    let mut created_files = Vec::new();
    let mut injected_contents = Vec::new();
    let mut customize_markers = Vec::new();

    for action in &def.actions {
        match action {
            generator::Action::Create { template, output } => {
                let rendered_output = render::render_expression(&tera, output, &ctx)?;
                let rendered_content = render::render_template(&tera, template, &ctx)?;
                let rel_path = RelativePath::new(&rendered_output)?;

                customize_markers.extend(markers::extract_ai_markers(&rel_path, &rendered_content));

                if dry_run {
                    if !json_output {
                        println!("  {} {}", label("create", Color::Green), rel_path);
                    }
                    created_files.push(manifest::ManifestCreatedFile {
                        path: rel_path,
                        template: template.to_string(),
                    });
                } else {
                    let result = file_ops::create_file(
                        &root,
                        &rel_path,
                        &rendered_content,
                        template,
                        force,
                    )?;
                    if let Some(hook) = &project_config.hooks.post_generate {
                        hook.run(&root.join(result.path.as_ref()));
                    }
                    if !json_output {
                        println!("  {} {}", label("create", Color::Green), result.path);
                    }
                    created_files.push(manifest::ManifestCreatedFile {
                        path: result.path,
                        template: result.template,
                    });
                }
            }
            generator::Action::Inject {
                target,
                marker,
                content,
            } => {
                handle_inject(
                    &root,
                    &tera,
                    &ctx,
                    target,
                    marker,
                    content,
                    &project_config,
                    conflict_mode,
                    dry_run,
                    json_output,
                    &mut injected_contents,
                )?;
            }
        }
    }

    let inputs: BTreeMap<String, serde_json::Value> = var_map
        .into_iter()
        .map(|(k, values)| {
            if values.len() == 1 {
                (
                    k,
                    serde_json::Value::String(values.into_iter().next().unwrap()),
                )
            } else {
                let arr: Vec<serde_json::Value> =
                    values.into_iter().map(serde_json::Value::String).collect();
                (k, serde_json::Value::Array(arr))
            }
        })
        .collect();

    let result = manifest::GenerationResult {
        generator: def.generator.name,
        timestamp: chrono::Utc::now().to_rfc3339(),
        inputs,
        created: created_files,
        injected: injected_contents,
        customize: customize_markers,
    };

    if json_output {
        println!("{}", serde_json::to_string_pretty(&result)?);
    }

    if !dry_run {
        let manifest_path = discovery::manifest_path(&root);
        manifest::write_manifest(&manifest_path, &result)?;
        if !json_output {
            println!(
                "\n  {} {}",
                label("manifest", Color::Blue),
                manifest_path.display()
            );
        }
    }

    Ok(())
}

fn handle_inject(
    root: &std::path::Path,
    tera: &tera::Tera,
    ctx: &tera::Context,
    target: &str,
    marker: &crate::types::MarkerName,
    content: &str,
    config: &config::ProjectConfig,
    conflict_mode: file_ops::ConflictMode,
    dry_run: bool,
    json_output: bool,
    injected: &mut Vec<manifest::ManifestInjectedContent>,
) -> Result<()> {
    let rendered_content = render::render_expression(tera, content, ctx)?;
    let rendered_target = render::render_expression(tera, target, ctx)?;
    let rel_path = RelativePath::new(&rendered_target)?;

    if dry_run {
        if !json_output {
            println!(
                "  {} {} (marker: {})",
                label("inject", Color::Yellow),
                rel_path,
                marker
            );
        }
        injected.push(manifest::ManifestInjectedContent {
            path: rel_path,
            marker: marker.clone(),
            content: rendered_content,
        });
        return Ok(());
    }

    let result = file_ops::inject_before_marker(
        root,
        &rel_path,
        marker,
        &rendered_content,
        &config.comment_style,
        conflict_mode,
    )?;

    if !json_output {
        if result.skipped {
            println!(
                "  {} {} (marker: {}, already present)",
                label("skip", Color::Cyan),
                result.path,
                result.marker
            );
        } else {
            println!(
                "  {} {} (marker: {})",
                label("inject", Color::Yellow),
                result.path,
                result.marker
            );
        }
    }

    injected.push(manifest::ManifestInjectedContent {
        path: result.path,
        marker: result.marker,
        content: result.content,
    });

    Ok(())
}

pub enum Color {
    Green,
    Yellow,
    Cyan,
    Blue,
}

pub fn label(text: &str, color: Color) -> String {
    if std::env::var_os("NO_COLOR").is_some() {
        return text.to_string();
    }
    match color {
        Color::Green => format!("{}", text.green().bold()),
        Color::Yellow => format!("{}", text.yellow().bold()),
        Color::Cyan => format!("{}", text.cyan().bold()),
        Color::Blue => format!("{}", text.blue().bold()),
    }
}

pub fn error_label() -> String {
    if std::env::var_os("NO_COLOR").is_some() {
        return "error:".to_string();
    }
    format!("{}", "error:".red().bold())
}
