use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

mod commands;

use commands::{BenchArgs, BuildArgs, DeployArgs, DevArgs, GenerateArgs, NewArgs, SchemaCommand, TraceArgs};

#[derive(Parser)]
#[command(
    name = "ag",
    about = "Anti-Gravital — high-performance Rust web framework",
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
    /// Generate Rust, TypeScript, and OpenAPI from the project schema.ag.
    Generate(GenerateArgs),
    /// Start the development server with live reload.
    Dev(DevArgs),
    /// Build a production binary.
    Build(BuildArgs),
    /// Validate and inspect the project schema.
    Schema {
        #[command(subcommand)]
        command: SchemaCommand,
    },
    /// Run a throughput benchmark against a running server.
    Bench(BenchArgs),
    /// Connect to the tokio-console async task inspector.
    Trace(TraceArgs),
    /// Deploy the application to Docker or Kubernetes.
    Deploy(DeployArgs),
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
        Command::Bench(args) => commands::bench::run(args).await,
        Command::Trace(args) => commands::trace::run(args).await,
        Command::Deploy(args) => commands::deploy::run(args).await,
    };

    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
