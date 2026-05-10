use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use thiserror::Error;
use tracing::info;

use crate::bus::MemoryBus;

/// Configuration for The Shield HTTP layer.
#[derive(Debug, Clone)]
pub struct ShieldConfig {
    /// Address to bind the HTTP listener.
    pub addr: SocketAddr,
    /// Path to the TLS certificate file (PEM). `None` disables TLS (dev mode only).
    pub tls_cert: Option<PathBuf>,
    /// Path to the TLS private key file (PEM).
    pub tls_key: Option<PathBuf>,
    /// Maximum number of concurrent connections.
    pub max_connections: usize,
    /// Per-request timeout in milliseconds.
    pub request_timeout_ms: u64,
    /// Maximum allowed request body size in bytes.
    pub max_body_bytes: usize,
}

impl Default for ShieldConfig {
    fn default() -> Self {
        Self {
            addr: "127.0.0.1:3000".parse().unwrap(),
            tls_cert: None,
            tls_key: None,
            max_connections: 65_535,
            request_timeout_ms: 30_000,
            max_body_bytes: 4 * 1024 * 1024,
        }
    }
}

#[derive(Debug, Error)]
pub enum ShieldError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("TLS configuration error: {0}")]
    Tls(String),
    #[error("Memory bus error: {0}")]
    Bus(#[from] crate::bus::BusError),
}

/// The Shield: the network-facing Rust layer of Anti-Gravital.
///
/// Responsibilities in the full implementation (Phase 1):
/// - TLS 1.3 termination via rustls
/// - HTTP/1.1 and HTTP/2 parsing via hyper
/// - Request schema validation against compiled `.ag` contracts
/// - JWT verification (Ed25519) and rate limiting
/// - Writing validated requests into the Memory Bus for Go processing
/// - Collecting responses from the Memory Bus and sending HTTP replies
///
/// In Phase 0, Shield is a placeholder that logs its configuration and returns
/// immediately. The public interface is stable.
pub struct Shield {
    config: ShieldConfig,
    bus: Arc<MemoryBus>,
}

impl Shield {
    /// Creates a new Shield instance bound to the given configuration and bus.
    pub fn new(config: ShieldConfig, bus: Arc<MemoryBus>) -> Self {
        Self { config, bus }
    }

    /// Starts the Shield listener. Blocks until the server shuts down.
    pub async fn run(self) -> Result<(), ShieldError> {
        info!(
            addr = %self.config.addr,
            max_connections = self.config.max_connections,
            tls = self.config.tls_cert.is_some(),
            "Shield starting"
        );
        info!(bus_capacity = self.bus.capacity(), "Memory bus connected");
        info!("Shield running (Phase 0 placeholder — HTTP listener not yet implemented)");
        Ok(())
    }

    /// Returns the address this Shield is configured to bind.
    pub fn addr(&self) -> SocketAddr {
        self.config.addr
    }
}
