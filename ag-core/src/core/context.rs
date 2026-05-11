use axum::{
    async_trait,
    body::Bytes,
    extract::{FromRequest, Request},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::de::DeserializeOwned;

use super::error::AgError;

/// Extractor that deserializes a JSON body and validates that the
/// `Content-Type` header is `application/json`.
///
/// Returns [`AgError::BadRequest`] on malformed JSON and
/// [`AgError::Validation`] when the body is empty where one is expected.
pub struct ValidatedBody<T>(pub T);

#[async_trait]
impl<T, S> FromRequest<S> for ValidatedBody<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let content_type = req
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        if !content_type.starts_with("application/json") {
            return Err(AgError::BadRequest(
                "Content-Type must be application/json".into(),
            )
            .into_response());
        }

        let bytes = Bytes::from_request(req, state)
            .await
            .map_err(|e| AgError::Internal(e.to_string()).into_response())?;

        if bytes.is_empty() {
            return Err(AgError::Validation("request body must not be empty".into()).into_response());
        }

        serde_json::from_slice::<T>(&bytes)
            .map(ValidatedBody)
            .map_err(|e| AgError::BadRequest(format!("invalid JSON: {e}")).into_response())
    }
}

/// Metadata extracted from the request by the Shield layer.
///
/// Available to handlers via `axum::extract::Extension<RequestContext>`.
#[derive(Clone, Debug, Default)]
pub struct RequestContext {
    /// The `x-request-id` header value set by the request-id middleware.
    pub request_id: Option<String>,
    /// Subject claim from a verified JWT, if authentication was performed.
    pub subject: Option<String>,
    /// Client IP address after proxy header resolution.
    pub client_ip: Option<std::net::IpAddr>,
}

impl RequestContext {
    pub fn from_headers(headers: &HeaderMap) -> Self {
        let request_id = headers
            .get("x-request-id")
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned);

        Self {
            request_id,
            subject: None,
            client_ip: None,
        }
    }
}

/// Extractor that pulls [`RequestContext`] from the extension map.
pub struct ExtractContext(pub RequestContext);

#[async_trait]
impl<S> FromRequest<S> for ExtractContext
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, Json<serde_json::Value>);

    async fn from_request(req: Request, _state: &S) -> Result<Self, Self::Rejection> {
        let ctx = req
            .extensions()
            .get::<RequestContext>()
            .cloned()
            .unwrap_or_default();
        Ok(ExtractContext(ctx))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    #[test]
    fn request_context_extracts_request_id() {
        let mut headers = HeaderMap::new();
        headers.insert("x-request-id", HeaderValue::from_static("abc-123"));
        let ctx = RequestContext::from_headers(&headers);
        assert_eq!(ctx.request_id.as_deref(), Some("abc-123"));
    }

    #[test]
    fn request_context_handles_missing_id() {
        let headers = HeaderMap::new();
        let ctx = RequestContext::from_headers(&headers);
        assert!(ctx.request_id.is_none());
    }
}
