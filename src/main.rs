mod commands;
mod config;
mod context;
mod discovery;
mod fields;
mod file_ops;
mod filters;
mod generate;
mod generator;
mod manifest;
mod markers;
mod render;

use anyhow::Result;
use clap::{Parser, Subcommand};

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
        eprintln!("{} {e:#}", generate::error_label());
        std::process::exit(1);
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
        } => generate::run(&name, &vars, force, skip_existing, dry_run, json),
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
        Commands::Init { lang } => commands::init::run(lang.as_deref()),
        Commands::Template { action } => {
            let cwd = std::env::current_dir()?;
            let root = discovery::find_jujo_root(&cwd)?;
            match action {
                TemplateCommands::Add { name, from, force } => {
                    commands::template::add(&root, &name, &from, force)
                }
                TemplateCommands::Remove { name } => commands::template::remove(&root, &name),
                TemplateCommands::List { json } => commands::list::run(&root, json),
            }
        }
    }
}
