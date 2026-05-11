use ag_core::{build_router, AppState, ServerConfig};
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use criterion::{criterion_group, criterion_main, Criterion};
use tower::ServiceExt;

fn bench_health_endpoint(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    c.bench_function("GET /health (single-threaded)", |b| {
        b.to_async(&rt).iter(|| async {
            let state = AppState::new(ServerConfig::default());
            let app = build_router(state);
            let req = Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap();
            let resp = app.oneshot(req).await.unwrap();
            assert_eq!(resp.status(), StatusCode::OK);
        });
    });
}

fn bench_unknown_route(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    c.bench_function("GET /unknown (404 path)", |b| {
        b.to_async(&rt).iter(|| async {
            let state = AppState::new(ServerConfig::default());
            let app = build_router(state);
            let req = Request::builder()
                .uri("/unknown")
                .body(Body::empty())
                .unwrap();
            let resp = app.oneshot(req).await.unwrap();
            assert_eq!(resp.status(), StatusCode::NOT_FOUND);
        });
    });
}

criterion_group!(benches, bench_health_endpoint, bench_unknown_route);
criterion_main!(benches);
