mod config;
mod context;
mod discovery;
mod fields;
mod file_ops;
mod filters;
mod generator;
mod manifest;
mod render;

use anyhow::{bail, Result};
use clap::{Parser, Subcommand};
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
        /// Overwrite existing files
        #[arg(long)]
        force: bool,
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
}

fn main() {
    let cli = Cli::parse();
    if let Err(e) = run(cli) {
        eprintln!("error: {e:#}");
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Generate { name, vars, force } => cmd_generate(&name, &vars, force),
        Commands::List { .. } => bail!("not yet implemented: list"),
        Commands::Describe { .. } => bail!("not yet implemented: describe"),
        Commands::Validate => bail!("not yet implemented: validate"),
        Commands::Init { .. } => bail!("not yet implemented: init"),
    }
}

fn cmd_generate(name: &str, vars: &[String], force: bool) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let root = discovery::find_jujo_root(&cwd)?;
    let gen_dir = discovery::generator_dir(&root, name)?;
    let project_config = config::load_config(&root)?;

    let def = generator::load_generator(&gen_dir)?;
    let var_map = context::parse_vars(vars)?;
    let ctx = context::build_context(&def, &var_map, &project_config)?;
    let tera = render::create_tera(&gen_dir)?;

    let mut created_files = Vec::new();

    for action in &def.actions {
        match action {
            generator::Action::Create { template, output } => {
                let rendered_output = render::render_expression(&tera, output, &ctx)?;
                let rendered_content = render::render_template(&tera, template, &ctx)?;
                let result = file_ops::create_file(
                    &root,
                    &rendered_output,
                    &rendered_content,
                    template,
                    force,
                )?;
                println!("  create {}", result.path);
                created_files.push(manifest::ManifestCreatedFile {
                    path: result.path,
                    template: result.template,
                });
            }
            generator::Action::Inject { .. } => {
                // Phase 3: injection not yet implemented.
                eprintln!("  skip   inject actions not yet implemented");
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
                let arr: Vec<serde_json::Value> = values
                    .into_iter()
                    .map(serde_json::Value::String)
                    .collect();
                (k, serde_json::Value::Array(arr))
            }
        })
        .collect();

    let result = manifest::GenerationResult {
        generator: def.generator.name.clone(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        inputs,
        created: created_files,
        injected: vec![],
        customize: vec![],
    };

    let manifest_path = discovery::manifest_path(&root);
    manifest::write_manifest(&manifest_path, &result)?;
    println!("\n  manifest {}", manifest_path.display());

    Ok(())
}
