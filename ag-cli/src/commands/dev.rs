use std::sync::Arc;

use clap::Args;
use tracing::info;

use ag_core::bus::MemoryBus;
use ag_core::shield::{Shield, ShieldConfig};

#[derive(Args)]
pub struct DevArgs {
    /// Host to bind the development server.
    #[arg(long, default_value = "127.0.0.1")]
    host: String,

    /// Port to bind the development server.
    #[arg(short, long, default_value = "3000")]
    port: u16,

    /// Path to schema file.
    #[arg(long, default_value = "schema.ag")]
    schema: String,
}

pub async fn run(args: DevArgs) -> anyhow::Result<()> {
    let addr: std::net::SocketAddr = format!("{}:{}", args.host, args.port)
        .parse()
        .map_err(|e| anyhow::anyhow!("invalid address: {e}"))?;

    info!("Starting development server on http://{addr}");
    info!("Documentation: http://{addr}/docs");
    info!("Metrics:       http://{addr}/metrics");
    info!("Press Ctrl+C to stop");

    let bus = Arc::new(MemoryBus::create(ag_core::BUS_DEFAULT_NAME)?);

    let config = ShieldConfig {
        addr,
        tls_cert: None,
        tls_key: None,
        ..Default::default()
    };
    let shield = Shield::new(config, Arc::clone(&bus));
    shield.run().await?;

    Ok(())
}
