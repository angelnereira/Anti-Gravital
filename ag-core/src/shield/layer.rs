use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use axum::extract::Request;
use axum::response::Response;
use tower::{Layer, Service};

/// Tower [`Layer`] that applies Shield validation to every request.
///
/// In Phase 1+ this layer will enforce:
/// - Schema validation against compiled `.ag` contracts
/// - JWT / Ed25519 signature verification
/// - Rate limiting (token-bucket per client IP)
/// - Maximum body size enforcement
///
/// In the current phase it is a zero-cost pass-through that establishes
/// the stable public interface for downstream middleware composition.
#[derive(Debug, Clone, Default)]
pub struct ShieldLayer;

impl<S> Layer<S> for ShieldLayer {
    type Service = ShieldService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        ShieldService { inner }
    }
}

/// The service produced by [`ShieldLayer`].
#[derive(Debug, Clone)]
pub struct ShieldService<S> {
    inner: S,
}

impl<S> Service<Request> for ShieldService<S>
where
    S: Service<Request, Response = Response> + Clone + Send + 'static,
    S::Future: Send + 'static,
    S::Error: Send,
{
    type Response = Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request) -> Self::Future {
        // Clone inner to satisfy the owned-service pattern required by Tower.
        let clone = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, clone);

        Box::pin(async move {
            // Phase 1+: insert JWT verification, schema validation, and rate-limiting here
            // before forwarding to the Core handler layer.
            inner.call(req).await
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use axum::response::Response;
    use tower::{service_fn, Service, ServiceExt};

    #[tokio::test]
    async fn shield_passes_request_through() {
        let inner = service_fn(|_req: Request<Body>| async {
            Ok::<Response<Body>, std::convert::Infallible>(
                Response::builder()
                    .status(StatusCode::OK)
                    .body(Body::empty())
                    .unwrap(),
            )
        });

        let mut svc = ShieldLayer.layer(inner);
        let req = Request::builder().uri("/test").body(Body::empty()).unwrap();
        let resp = <_ as Service<Request<Body>>>::call(svc.ready().await.unwrap(), req)
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }
}
