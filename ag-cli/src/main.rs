use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

mod commands;

use commands::{BuildArgs, DevArgs, GenerateArgs, NewArgs, SchemaCommand};

#[derive(Parser)]
#[command(
    name = "ag",
    about = "Anti-Gravital — a high-performance Rust+Go web framework",
    version,
    long_about = None,
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Create a new Anti-Gravital project.
    New(NewArgs),
    /// Generate code from the project schema.ag.
    Generate(GenerateArgs),
    /// Start the development server with hot reload.
    Dev(DevArgs),
    /// Build a production binary.
    Build(BuildArgs),
    /// Validate and inspect the project schema.
    Schema {
        #[command(subcommand)]
        command: SchemaCommand,
    },
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .without_time()
        .with_target(false)
        .init();

    let cli = Cli::parse();

    let result = match cli.command {
        Command::New(args) => commands::new::run(args).await,
        Command::Generate(args) => commands::generate::run(args).await,
        Command::Dev(args) => commands::dev::run(args).await,
        Command::Build(args) => commands::build::run(args).await,
        Command::Schema { command } => commands::schema::run(command).await,
    };

    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
