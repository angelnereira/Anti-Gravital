use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AgError {
    #[error("not found: {0}")]
    NotFound(String),

    #[error("bad request: {0}")]
    BadRequest(String),

    #[error("validation failed: {0}")]
    Validation(String),

    #[error("unauthorized")]
    Unauthorized,

    #[error("forbidden")]
    Forbidden,

    #[error("internal error: {0}")]
    Internal(String),
}

impl IntoResponse for AgError {
    fn into_response(self) -> Response {
        let (status, code) = match &self {
            AgError::NotFound(_) => (StatusCode::NOT_FOUND, "NOT_FOUND"),
            AgError::BadRequest(_) => (StatusCode::BAD_REQUEST, "BAD_REQUEST"),
            AgError::Validation(_) => (StatusCode::UNPROCESSABLE_ENTITY, "VALIDATION_ERROR"),
            AgError::Unauthorized => (StatusCode::UNAUTHORIZED, "UNAUTHORIZED"),
            AgError::Forbidden => (StatusCode::FORBIDDEN, "FORBIDDEN"),
            AgError::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR"),
        };

        let message = self.to_string();
        let body = Json(json!({
            "error": {
                "code": code,
                "message": message,
            }
        }));

        (status, body).into_response()
    }
}

pub type AgResult<T> = Result<T, AgError>;

impl From<anyhow::Error> for AgError {
    fn from(e: anyhow::Error) -> Self {
        AgError::Internal(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;

    #[test]
    fn not_found_maps_to_404() {
        let err = AgError::NotFound("user".into());
        let resp = err.into_response();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn unauthorized_maps_to_401() {
        let err = AgError::Unauthorized;
        let resp = err.into_response();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn validation_maps_to_422() {
        let err = AgError::Validation("field 'email' is required".into());
        let resp = err.into_response();
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }
}
