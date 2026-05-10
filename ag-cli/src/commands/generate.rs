use std::fs;
use std::path::Path;

use clap::Args;
use tracing::info;

#[derive(Args)]
pub struct GenerateArgs {
    /// Path to the schema file.
    #[arg(long, default_value = "schema.ag")]
    schema: String,

    /// Output directory for generated files.
    #[arg(long, default_value = "src")]
    out_dir: String,
}

pub async fn run(args: GenerateArgs) -> anyhow::Result<()> {
    let schema_path = Path::new(&args.schema);
    if !schema_path.exists() {
        anyhow::bail!("schema file '{}' not found", args.schema);
    }

    let source = fs::read_to_string(schema_path)?;
    info!("Parsing schema from '{}'", args.schema);

    match ag_core::dsl::parse(&source) {
        Ok(_schema) => {
            info!("Schema parsed successfully");
            println!("Would generate:");
            println!("  {}/rust/models.rs", args.out_dir);
            println!("  {}/rust/validators.rs", args.out_dir);
            println!("  {}/go/models.go", args.out_dir);
            println!("  {}/go/handlers_stubs.go", args.out_dir);
            println!("  {}/go/queries.sql.go", args.out_dir);
            println!("  {}/ts/types.ts", args.out_dir);
            println!("  {}/ts/client.ts", args.out_dir);
            println!("  openapi.yaml");
        }
        Err(ag_core::dsl::DslError::Parse { .. }) => {
            println!("Note: The Anti-DSL parser is scheduled for Phase 3.");
            println!("Schema file '{}' was found and read ({} bytes).", args.schema, source.len());
            println!("Code generation will be available once the parser is implemented.");
        }
        Err(e) => return Err(e.into()),
    }

    Ok(())
}
