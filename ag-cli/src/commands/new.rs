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
    Fullstack,
    Realtime,
}

impl std::fmt::Display for Template {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Template::Rest => write!(f, "rest"),
            Template::Fullstack => write!(f, "fullstack"),
            Template::Realtime => write!(f, "realtime"),
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
    fs::create_dir_all(dir.join("src/db/migrations"))?;
    fs::create_dir_all(dir.join("ts"))?;

    // schema.ag
    let schema = format!(
        r#"model HealthCheck {{
  status  String
  version String
  uptime  Int
}}

endpoint Health {{
  method   GET
  path     /health
  response HealthCheck
}}
"#
    );
    fs::write(dir.join("schema.ag"), schema)?;

    // Handler stub (the developer fills the body)
    let handler = r#"use axum::Json;
use ag_core::core::error::AgResult;

use crate::models::HealthCheck;

pub async fn health() -> AgResult<Json<HealthCheck>> {
    Ok(Json(HealthCheck {
        status: "ok".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        uptime: 0,
    }))
}
"#;
    fs::write(dir.join("src/handlers/health.rs"), handler)?;

    // Cargo.toml for the generated project
    let cargo_toml = format!(
        r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2021"

[dependencies]
ag-core = "0.1"
tokio = {{ version = "1", features = ["full"] }}
axum = "0.7"
serde = {{ version = "1", features = ["derive"] }}
serde_json = "1"
"#
    );
    fs::write(dir.join("Cargo.toml"), cargo_toml)?;

    // .gitignore
    fs::write(
        dir.join(".gitignore"),
        "target/\ndist/\n*.ag.generated/\n",
    )?;

    println!("Created project '{name}'");
    println!();
    println!("  cd {name}");
    println!("  ag generate    # generate Rust, TypeScript, and OpenAPI from schema.ag");
    println!("  ag dev         # start the development server");
    println!("  ag build       # build a single static binary for production");
    Ok(())
}
