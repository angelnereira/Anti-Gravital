use clap::{Args, ValueEnum};
use tracing::info;

#[derive(ValueEnum, Clone, Debug)]
pub enum DeployTarget {
    Docker,
    Kubernetes,
}

#[derive(Args)]
pub struct DeployArgs {
    /// Deployment target platform.
    #[arg(long, value_enum, default_value = "docker")]
    to: DeployTarget,

    /// Image tag for Docker/Kubernetes deployments.
    #[arg(long, default_value = "latest")]
    tag: String,

    /// Namespace for Kubernetes deployments.
    #[arg(long, default_value = "default")]
    namespace: String,
}

pub async fn run(args: DeployArgs) -> anyhow::Result<()> {
    let target = match args.to {
        DeployTarget::Docker => "Docker",
        DeployTarget::Kubernetes => "Kubernetes",
    };

    info!(target, tag = %args.tag, "Preparing deployment");

    println!("Deployment target: {target}");
    println!("Image tag: {}", args.tag);
    println!();
    println!("Anti-Gravital deploy is scheduled for Phase 3.");
    println!("In the meantime, build a release binary with `ag build` and package it manually.");

    Ok(())
}
