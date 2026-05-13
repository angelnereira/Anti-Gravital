use std::fs;
use std::path::Path;

use clap::Args;
use tracing::info;

use ag_dsl::codegen;

#[derive(Args)]
pub struct GenerateArgs {
    /// Path to the schema file.
    #[arg(long, default_value = "schema.ag")]
    schema: String,

    /// Output directory for generated files.
    #[arg(long, default_value = ".")]
    out_dir: String,

    /// Preview which files would be written without actually writing them.
    #[arg(long)]
    dry_run: bool,
}

pub async fn run(args: GenerateArgs) -> anyhow::Result<()> {
    let schema_path = Path::new(&args.schema);
    if !schema_path.exists() {
        anyhow::bail!("schema file '{}' not found", args.schema);
    }

    let source = fs::read_to_string(schema_path)?;
    info!("Compiling schema from '{}'", args.schema);

    let schema = ag_dsl::compile(&source)
        .map_err(|e| anyhow::anyhow!("DSL error: {e}"))?;

    info!(
        models = schema.models.len(),
        endpoints = schema.endpoints.len(),
        "Schema compiled"
    );

    let files = codegen::generate_all(&schema)
        .map_err(|e| anyhow::anyhow!("codegen error: {e}"))?;

    if args.dry_run {
        println!("Dry run — no files written. Would generate:");
        for f in &files {
            println!("  {}/{}", args.out_dir, f.path);
        }
        println!("\nRemove --dry-run to write files to disk.");
    } else {
        let out = Path::new(&args.out_dir);
        for f in &files {
            let dest = out.join(&f.path);
            if let Some(parent) = dest.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&dest, &f.content)?;
            println!("Written {}", dest.display());
            info!("Written {}", dest.display());
        }
        println!("\nGenerated {} file(s).", files.len());
    }

    Ok(())
}
