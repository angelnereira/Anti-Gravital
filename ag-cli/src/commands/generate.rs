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
    #[arg(long, default_value = "src")]
    out_dir: String,

    /// Write generated files to disk instead of printing a summary.
    #[arg(long)]
    write: bool,
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

    if args.write {
        let out = Path::new(&args.out_dir);
        for f in &files {
            let dest = out.join(&f.path);
            if let Some(parent) = dest.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&dest, &f.content)?;
            info!("Written {}", dest.display());
        }
    } else {
        println!("Would generate:");
        for f in &files {
            println!("  {}/{}", args.out_dir, f.path);
        }
        println!("\nRun with --write to write files to disk.");
    }

    Ok(())
}
