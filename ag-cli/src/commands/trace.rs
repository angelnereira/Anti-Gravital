use clap::Args;
use tracing::info;

#[derive(Args)]
pub struct TraceArgs {
    /// tokio-console server address to connect to.
    #[arg(long, default_value = "127.0.0.1:6669")]
    console_addr: String,
}

pub async fn run(args: TraceArgs) -> anyhow::Result<()> {
    info!(console_addr = %args.console_addr, "Connecting to tokio-console");

    println!("tokio-console integration is scheduled for Phase 2.");
    println!();
    println!(
        "Start the console server by launching your app with TOKIO_CONSOLE_BIND={} \
         and the `tokio-console` subscriber enabled.",
        args.console_addr
    );
    println!("Then run: tokio-console http://{}", args.console_addr);

    Ok(())
}
