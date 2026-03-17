mod commands;
mod config;
mod context;
mod discovery;
mod fields;
mod file_ops;
mod filters;
mod generator;
mod manifest;
mod markers;
mod render;

use anyhow::{bail, Result};
use clap::{Parser, Subcommand};
use owo_colors::OwoColorize;
use std::collections::BTreeMap;

#[derive(Parser)]
#[command(name = "jujo", about = "Agent-first code generation framework")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate files from a template set
    Generate {
        /// Generator name (directory under .jujo/templates/)
        name: String,
        /// Input variables as key=value pairs
        #[arg(long = "var", value_name = "KEY=VALUE")]
        vars: Vec<String>,
        /// Overwrite existing files and inject despite conflicts
        #[arg(long)]
        force: bool,
        /// Skip injection when content already present
        #[arg(long)]
        skip_existing: bool,
        /// Preview what would happen without writing files
        #[arg(long)]
        dry_run: bool,
        /// Output as JSON instead of human-readable text
        #[arg(long)]
        json: bool,
    },
    /// List available generators
    List {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Show generator schema
    Describe {
        /// Generator name
        name: String,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Validate all generators and templates
    Validate,
    /// Initialize a new .jujo/ directory
    Init {
        /// Language for type map (e.g., rust, go, python)
        #[arg(long)]
        lang: Option<String>,
    },
    /// Manage template sets
    Template {
        #[command(subcommand)]
        action: TemplateCommands,
    },
}

#[derive(Subcommand)]
enum TemplateCommands {
    /// Add a template set from a local path
    Add {
        /// Template name
        name: String,
        /// Source directory path
        #[arg(long)]
        from: String,
        /// Overwrite if exists
        #[arg(long)]
        force: bool,
    },
    /// Remove a template set
    Remove {
        /// Template name
        name: String,
    },
    /// List template sets (alias for `jujo list`)
    List {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}

fn main() {
    let cli = Cli::parse();
    if let Err(e) = run(cli) {
        eprintln!("{} {e:#}", label("error:", LabelColor::Red));
        std::process::exit(1);
    }
}

enum LabelColor { Green, Yellow, Cyan, Blue, Red }

fn label(text: &str, color: LabelColor) -> String {
    if std::env::var_os("NO_COLOR").is_some() {
        return text.to_string();
    }
    match color {
        LabelColor::Green => format!("{}", text.green().bold()),
        LabelColor::Yellow => format!("{}", text.yellow().bold()),
        LabelColor::Cyan => format!("{}", text.cyan().bold()),
        LabelColor::Blue => format!("{}", text.blue().bold()),
        LabelColor::Red => format!("{}", text.red().bold()),
    }
}

fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Generate {
            name,
            vars,
            force,
            skip_existing,
            dry_run,
            json,
        } => cmd_generate(&name, &vars, force, skip_existing, dry_run, json),
        Commands::List { json } => {
            let cwd = std::env::current_dir()?;
            let root = discovery::find_jujo_root(&cwd)?;
            commands::list::run(&root, json)
        }
        Commands::Describe { name, json } => {
            let cwd = std::env::current_dir()?;
            let root = discovery::find_jujo_root(&cwd)?;
            commands::describe::run(&root, &name, json)
        }
        Commands::Validate => {
            let cwd = std::env::current_dir()?;
            let root = discovery::find_jujo_root(&cwd)?;
            commands::validate::run(&root)
        }
        Commands::Init { lang } => {
            commands::init::run(lang.as_deref())
        }
        Commands::Template { action } => {
            let cwd = std::env::current_dir()?;
            let root = discovery::find_jujo_root(&cwd)?;
            match action {
                TemplateCommands::Add { name, from, force } => {
                    commands::template::add(&root, &name, &from, force)
                }
                TemplateCommands::Remove { name } => {
                    commands::template::remove(&root, &name)
                }
                TemplateCommands::List { json } => {
                    commands::list::run(&root, json)
                }
            }
        }
    }
}

fn cmd_generate(
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

    // --force applies to file creation (overwrite). Injection conflict is separate:
    // --skip-existing silently skips, otherwise error on conflict.
    // --force only forces injection if --skip-existing is not set.
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

                // Extract AI customization markers from rendered content.
                customize_markers.extend(
                    markers::extract_ai_markers(&rendered_output, &rendered_content),
                );

                if dry_run {
                    if !json_output {
                        println!("  {} {}", label("create", LabelColor::Green), rendered_output);
                    }
                    created_files.push(manifest::ManifestCreatedFile {
                        path: rendered_output,
                        template: template.clone(),
                    });
                } else {
                    let result = file_ops::create_file(
                        &root,
                        &rendered_output,
                        &rendered_content,
                        template,
                        force,
                    )?;
                    if !json_output {
                        println!("  {} {}", label("create", LabelColor::Green), result.path);
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
                let rendered_content = render::render_expression(&tera, content, &ctx)?;
                let rendered_target = render::render_expression(&tera, target, &ctx)?;

                if dry_run {
                    if !json_output {
                        println!(
                            "  {} {} (marker: {})",
                            label("inject", LabelColor::Yellow),
                            rendered_target,
                            marker
                        );
                    }
                    injected_contents.push(manifest::ManifestInjectedContent {
                        path: rendered_target,
                        marker: marker.clone(),
                        content: rendered_content,
                    });
                } else {
                    let result = file_ops::inject_before_marker(
                        &root,
                        &rendered_target,
                        marker,
                        &rendered_content,
                        &project_config.comment_prefix,
                        &project_config.comment_suffix,
                        conflict_mode,
                    )?;
                    if !json_output {
                        if result.skipped {
                            println!(
                                "  {} {} (marker: {}, already present)",
                                label("skip", LabelColor::Cyan),
                                result.path,
                                result.marker
                            );
                        } else {
                            println!(
                                "  {} {} (marker: {})",
                                label("inject", LabelColor::Yellow),
                                result.path,
                                result.marker
                            );
                        }
                    }
                    injected_contents.push(manifest::ManifestInjectedContent {
                        path: result.path,
                        marker: result.marker,
                        content: result.content,
                    });
                }
            }
        }
    }

    // Build inputs map for the manifest.
    let inputs: BTreeMap<String, serde_json::Value> = var_map
        .into_iter()
        .map(|(k, values)| {
            if values.len() == 1 {
                (k, serde_json::Value::String(values.into_iter().next().unwrap()))
            } else {
                let arr: Vec<serde_json::Value> =
                    values.into_iter().map(serde_json::Value::String).collect();
                (k, serde_json::Value::Array(arr))
            }
        })
        .collect();

    let result = manifest::GenerationResult {
        generator: def.generator.name.clone(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        inputs,
        created: created_files,
        injected: injected_contents,
        customize: customize_markers,
    };

    if json_output {
        let json = serde_json::to_string_pretty(&result)?;
        println!("{json}");
    } else if !dry_run {
        let manifest_path = discovery::manifest_path(&root);
        manifest::write_manifest(&manifest_path, &result)?;
        println!("\n  {} {}", label("manifest", LabelColor::Blue), manifest_path.display());
    }

    // Write manifest even in non-JSON mode (but not in dry-run).
    if !dry_run && json_output {
        let manifest_path = discovery::manifest_path(&root);
        manifest::write_manifest(&manifest_path, &result)?;
    }

    Ok(())
}
