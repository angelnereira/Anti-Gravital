use std::net::SocketAddr;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub addr: SocketAddr,
    pub request_timeout: Duration,
    pub max_body_bytes: usize,
    pub cors_origins: Vec<String>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            addr: "127.0.0.1:3000".parse().unwrap(),
            request_timeout: Duration::from_secs(30),
            max_body_bytes: 4 * 1024 * 1024,
            cors_origins: vec![],
        }
    }
}

impl ServerConfig {
    pub fn builder() -> ServerConfigBuilder {
        ServerConfigBuilder::default()
    }
}

#[derive(Default)]
pub struct ServerConfigBuilder {
    addr: Option<SocketAddr>,
    request_timeout: Option<Duration>,
    max_body_bytes: Option<usize>,
    cors_origins: Vec<String>,
}

impl ServerConfigBuilder {
    pub fn addr(mut self, addr: SocketAddr) -> Self {
        self.addr = Some(addr);
        self
    }

    pub fn request_timeout(mut self, timeout: Duration) -> Self {
        self.request_timeout = Some(timeout);
        self
    }

    pub fn max_body_bytes(mut self, bytes: usize) -> Self {
        self.max_body_bytes = Some(bytes);
        self
    }

    pub fn cors_origin(mut self, origin: impl Into<String>) -> Self {
        self.cors_origins.push(origin.into());
        self
    }

    pub fn build(self) -> ServerConfig {
        let defaults = ServerConfig::default();
        ServerConfig {
            addr: self.addr.unwrap_or(defaults.addr),
            request_timeout: self.request_timeout.unwrap_or(defaults.request_timeout),
            max_body_bytes: self.max_body_bytes.unwrap_or(defaults.max_body_bytes),
            cors_origins: self.cors_origins,
        }
    }
}
