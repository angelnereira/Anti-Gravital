use std::fs;
use std::path::Path;

use clap::Subcommand;

#[derive(Subcommand)]
pub enum SchemaCommand {
    /// Validate the schema file for syntax and semantic errors.
    Lint {
        #[arg(long, default_value = "schema.ag")]
        schema: String,
    },
    /// Preview the database migrations that would be generated.
    Diff {
        #[arg(long, default_value = "schema.ag")]
        schema: String,
    },
    /// Apply pending database migrations.
    Migrate {
        #[arg(long, default_value = "schema.ag")]
        schema: String,
        #[arg(long)]
        dry_run: bool,
    },
}

pub async fn run(cmd: SchemaCommand) -> anyhow::Result<()> {
    match cmd {
        SchemaCommand::Lint { schema } => lint(&schema).await,
        SchemaCommand::Diff { schema } => diff(&schema).await,
        SchemaCommand::Migrate { schema, dry_run } => migrate(&schema, dry_run).await,
    }
}

async fn lint(schema_path: &str) -> anyhow::Result<()> {
    let path = Path::new(schema_path);
    if !path.exists() {
        anyhow::bail!("schema file '{schema_path}' not found");
    }

    let source = fs::read_to_string(path)?;
    println!("Linting '{schema_path}' ({} bytes)...", source.len());

    match ag_core::dsl::parse(&source) {
        Ok(schema) => {
            match ag_core::dsl::validate(&schema) {
                Ok(()) => println!("No issues found."),
                Err(errors) => {
                    for e in &errors {
                        eprintln!("  error: {e}");
                    }
                    anyhow::bail!("{} semantic error(s)", errors.len());
                }
            }
        }
        Err(ag_core::dsl::DslError::Parse { .. }) => {
            println!("Note: Anti-DSL parser is scheduled for Phase 3.");
            println!("Basic file checks passed (file exists and is readable).");
        }
        Err(e) => return Err(e.into()),
    }

    Ok(())
}

async fn diff(schema_path: &str) -> anyhow::Result<()> {
    let path = Path::new(schema_path);
    if !path.exists() {
        anyhow::bail!("schema file '{schema_path}' not found");
    }

    println!("Computing diff for '{schema_path}'...");
    println!("Note: Migration generation is scheduled for Phase 3.");
    Ok(())
}

async fn migrate(schema_path: &str, dry_run: bool) -> anyhow::Result<()> {
    let path = Path::new(schema_path);
    if !path.exists() {
        anyhow::bail!("schema file '{schema_path}' not found");
    }

    if dry_run {
        println!("Dry run: no migrations applied.");
    } else {
        println!("Computing migrations for '{schema_path}'...");
        println!("Note: Migration application is scheduled for Phase 3.");
    }
    Ok(())
}
