use std::sync::Arc;

use axum::{http::HeaderValue, routing::get, Json, Router};
use serde_json::json;
use tower::ServiceBuilder;
use tower_http::{
    compression::CompressionLayer,
    cors::CorsLayer,
    request_id::{MakeRequestUuid, SetRequestIdLayer},
    timeout::TimeoutLayer,
    trace::TraceLayer,
};
use tracing::info;

use crate::config::ServerConfig;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<ServerConfig>,
    pub started_at: std::time::Instant,
}

impl AppState {
    pub fn new(config: ServerConfig) -> Self {
        Self {
            config: Arc::new(config),
            started_at: std::time::Instant::now(),
        }
    }
}

/// Builds the Axum router with the full Tower middleware stack.
///
/// Middleware order (outermost → innermost):
///   request-id → trace → timeout → compression → cors → handlers
pub fn build_router(state: AppState) -> Router {
    let timeout = state.config.request_timeout;
    let cors = build_cors(&state.config.cors_origins);

    let middleware = ServiceBuilder::new()
        .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
        .layer(TraceLayer::new_for_http())
        .layer(TimeoutLayer::new(timeout))
        .layer(CompressionLayer::new())
        .layer(cors);

    Router::new()
        .route("/health", get(health_handler))
        .route("/metrics", get(metrics_handler))
        .layer(middleware)
        .with_state(state)
}

/// Binds a TCP listener and runs the server until shutdown.
pub async fn start_server(config: ServerConfig) -> Result<(), std::io::Error> {
    let addr = config.addr;
    let state = AppState::new(config);
    let app = build_router(state);

    info!(addr = %addr, "Anti-Gravital server starting");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!(addr = %addr, "listening");

    axum::serve(listener, app).await
}

fn build_cors(origins: &[String]) -> CorsLayer {
    if origins.is_empty() {
        CorsLayer::permissive()
    } else {
        let mut layer = CorsLayer::new();
        for origin in origins {
            if let Ok(hv) = origin.parse::<HeaderValue>() {
                layer = layer.allow_origin(hv);
            }
        }
        layer
    }
}

async fn health_handler(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> Json<serde_json::Value> {
    let uptime = state.started_at.elapsed().as_secs();
    Json(json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
        "uptime": uptime,
    }))
}

async fn metrics_handler() -> Json<serde_json::Value> {
    Json(json!({
        "requests_total": 0,
        "errors_total": 0,
        "latency_p99_ms": 0,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;

    fn test_state() -> AppState {
        AppState::new(ServerConfig::default())
    }

    #[tokio::test]
    async fn health_returns_ok() {
        let app = build_router(test_state());
        let req = Request::builder().uri("/health").body(Body::empty()).unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn unknown_route_returns_404() {
        let app = build_router(test_state());
        let req = Request::builder().uri("/nope").body(Body::empty()).unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn metrics_returns_ok() {
        let app = build_router(test_state());
        let req = Request::builder().uri("/metrics").body(Body::empty()).unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }
}

/// Convenience re-export used by benches.
pub use _bench::duration_30s;
mod _bench {
    use std::time::Duration;
    pub fn duration_30s() -> Duration {
        Duration::from_secs(30)
    }
}
