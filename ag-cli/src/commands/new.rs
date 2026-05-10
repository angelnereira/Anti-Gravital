use std::fs;
use std::path::Path;

use clap::Args;
use tracing::info;

#[derive(Args)]
pub struct NewArgs {
    /// Name of the project to create.
    name: String,

    /// Project template to use.
    #[arg(long, default_value = "rest")]
    template: Template,
}

#[derive(Clone, clap::ValueEnum)]
enum Template {
    Rest,
    Graphql,
    Grpc,
}

impl std::fmt::Display for Template {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Template::Rest => write!(f, "rest"),
            Template::Graphql => write!(f, "graphql"),
            Template::Grpc => write!(f, "grpc"),
        }
    }
}

pub async fn run(args: NewArgs) -> anyhow::Result<()> {
    let name = &args.name;
    let dir = Path::new(name);

    if dir.exists() {
        anyhow::bail!("directory '{name}' already exists");
    }

    info!("Creating project '{name}' from template '{}'", args.template);

    fs::create_dir_all(dir.join("src/handlers"))?;

    // schema.ag
    let namespace = name.replace('-', "_");
    let schema = format!(
        r#"@version 1.0
@namespace {namespace}

model HealthCheck {{
    status   String
    version  String
    uptime   Int
}}

endpoint Health {{
    method   GET
    path     /health
    response HealthCheck
}}
"#
    );
    fs::write(dir.join("schema.ag"), schema)?;

    // go.mod
    let go_mod = format!(
        "module github.com/example/{name}\n\ngo 1.21\n\nrequire github.com/gravital-labs/anti-gravital/ag-runtime v0.1.0\n"
    );
    fs::write(dir.join("go.mod"), go_mod)?;

    // Health handler stub
    let handler = r#"package handlers

import "github.com/gravital-labs/anti-gravital/ag-runtime/brain"

// HealthHandler implements the Health endpoint defined in schema.ag.
type HealthHandler struct{}

func (h *HealthHandler) Health(ctx brain.Context) error {
	return ctx.JSON(200, map[string]interface{}{
		"status":  "ok",
		"version": "0.1.0",
		"uptime":  0,
	})
}
"#;
    fs::write(dir.join("src/handlers/health.go"), handler)?;

    // .gitignore
    fs::write(dir.join(".gitignore"), "dist/\n*.ag.generated/\n")?;

    println!("Created project '{name}'");
    println!();
    println!("  cd {name}");
    println!("  ag generate    # generate code from schema.ag");
    println!("  ag dev         # start development server");
    Ok(())
}
