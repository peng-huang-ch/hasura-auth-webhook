use axum_extra::headers::HeaderMap;
use serde::{Deserialize, Serialize};

use crate::errors::{SrvError, SrvErrorKind};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApiKeyQuery {
    #[serde(rename = "x-api-key")]
    api_key: Option<String>,
}

#[allow(dead_code)]
pub fn api_key_validator(headers: HeaderMap, query: ApiKeyQuery) -> Result<String, SrvError> {
    // Extract API key from headers or query parameters
    // Check for `api-key` in headers
    if let Some(Some(key)) = headers.get("x-api-key").map(|v| v.to_str().ok()) {
        return Ok(key.to_string());
    }
    // Check for `api-key` in query parameters
    if let Some(key) = query.api_key {
        return Ok(key.to_string());
    }

    // Unauthorized if `x-api` is missing or invalid
    Err(SrvErrorKind::Unauthorized(
        "Invalid or missing API key".to_string(),
    ))?
}
