use axum::Router;

use crate::app::AppState;
use crate::shield::ShieldLayer;

/// Wraps an application [`Router`] with the Shield middleware layer.
///
/// This is the recommended way to attach Shield to any sub-router so that
/// schema validation and auth enforcement are applied uniformly.
///
/// ```rust,no_run
/// use ag_core::core::router::with_shield;
/// use ag_core::app::AppState;
/// use axum::{routing::get, Router};
///
/// async fn my_handler() -> &'static str { "ok" }
///
/// let inner = Router::new().route("/api/ping", get(my_handler));
/// let protected = with_shield(inner);
/// ```
pub fn with_shield(router: Router<AppState>) -> Router<AppState> {
    router.layer(ShieldLayer)
}
