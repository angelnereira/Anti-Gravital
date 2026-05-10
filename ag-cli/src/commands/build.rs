use std::process::Command;

use clap::Args;
use tracing::info;

#[derive(Args)]
pub struct BuildArgs {
    /// Compilation target triple (defaults to the host target).
    #[arg(long)]
    target: Option<String>,

    /// Output directory for the compiled binary.
    #[arg(long, default_value = "dist")]
    out_dir: String,

    /// Disable link-time optimisation (faster builds during iteration).
    #[arg(long)]
    no_lto: bool,
}

pub async fn run(args: BuildArgs) -> anyhow::Result<()> {
    let target_label = args
        .target
        .as_deref()
        .unwrap_or("host");

    info!("Building Anti-Gravital project for target: {target_label}");
    info!("Output directory: {}", args.out_dir);

    std::fs::create_dir_all(&args.out_dir)?;

    let mut cmd = Command::new("cargo");
    cmd.arg("build").arg("--release").arg("-p").arg("ag");

    if let Some(ref target) = args.target {
        cmd.arg("--target").arg(target);
    }

    info!("Running: {:?}", cmd);

    let status = cmd.status()?;
    if !status.success() {
        anyhow::bail!("cargo build failed with exit code {}", status);
    }

    info!("Build complete. Binary available in {}", args.out_dir);
    Ok(())
}
