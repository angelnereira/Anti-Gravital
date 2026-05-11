use clap::Args;
use tracing::info;

#[derive(Args)]
pub struct BenchArgs {
    /// Target endpoint path to benchmark.
    #[arg(long, default_value = "/health")]
    endpoint: String,

    /// Number of concurrent connections.
    #[arg(long, default_value = "100")]
    concurrency: usize,

    /// Total number of requests to send.
    #[arg(long, default_value = "10000")]
    requests: usize,

    /// Server address to target.
    #[arg(long, default_value = "http://127.0.0.1:3000")]
    url: String,
}

pub async fn run(args: BenchArgs) -> anyhow::Result<()> {
    info!(
        url = %args.url,
        endpoint = %args.endpoint,
        concurrency = args.concurrency,
        requests = args.requests,
        "Starting benchmark"
    );

    println!("Benchmarking {} requests to {}{}", args.requests, args.url, args.endpoint);
    println!("Concurrency: {} connections", args.concurrency);
    println!();
    println!("Anti-Gravital bench runner is scheduled for Phase 2.");
    println!("Use `cargo bench` to run criterion benchmarks against the in-process router.");

    Ok(())
}
