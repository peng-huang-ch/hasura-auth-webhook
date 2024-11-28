use std::collections::HashMap;

use axum::{extract::Query, http::StatusCode, response::Json};
use serde::Deserialize;
use serde_json::{json, Value};
use tracing::debug;

use crate::errors::{SrvError, SrvErrorKind};
use crate::validator::ApiKeyQuery;

#[derive(Debug, Deserialize)]
pub struct Consumer {
    #[serde(rename = "id")]
    id: Option<String>,
    #[serde(rename = "custom_id")]
    custom_id: Option<String>,
}

impl Consumer {
    fn is_valid(&self) -> bool {
        self.id.is_some() || self.custom_id.is_some()
    }
}

#[tracing::instrument]
async fn get_profile(api_key: String) -> Result<Value, SrvError> {
    let base_url = std::env::var("KONG_URL").map_err(|_| {
        SrvErrorKind::Custom(
            StatusCode::INTERNAL_SERVER_ERROR,
            "KONG_URL is not set".into(),
        )
    })?;

    let url = format!("{}/key-auths/{}/consumer", base_url, api_key);
    debug!("Fetching consumer from: {}", url);
    let consumer = reqwest::get(&url).await?.json::<Consumer>().await?;
    if !consumer.is_valid() {
        return Err(
            SrvErrorKind::Custom(StatusCode::UNAUTHORIZED, "Invalid API key".into()).into(),
        );
    }
    Ok(json!({
        "X-Hasura-User-Id": consumer.id.unwrap_or_default(),
        "X-Hasura-Role": "user",
        "X-Hasura-Is-Owner": "false",
        "X-Hasura-Custom": consumer.custom_id.unwrap_or_default(),
    }))
}

#[tracing::instrument(skip(query))]
pub async fn validate_request(
    Query(query): Query<ApiKeyQuery>,
    Json(payload): Json<HashMap<String, HashMap<String, String>>>,
) -> Result<Json<Value>, SrvError> {
    let headers = payload.get("headers").ok_or(SrvErrorKind::Custom(
        StatusCode::UNAUTHORIZED,
        "headers are required".into(),
    ))?;
    let token = headers.get("authorization").ok_or(SrvErrorKind::Custom(
        StatusCode::UNAUTHORIZED,
        "authorization is required".into(),
    ))?;
    let token = token.split("Bearer ").nth(1).unwrap_or_default();
    debug!(
        token = token,
        body = format!("{:?}", payload),
        query = format!("{:?}", query),
        "receiving request"
    );
    let profile = get_profile(token.to_string()).await?;
    Ok(Json(profile))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracing::info;

    #[tokio::test]
    async fn test_get_profile() {
        dotenvy::dotenv().ok();
        let api_key = "R78FanFgeJ7Wm63gvopqOf8MswEwepeN".to_string();
        let profile = get_profile(api_key).await.unwrap();
        assert!(profile.is_object());
        info!("Test profile: {:?}", profile);
    }

    #[test]
    fn test_consumer_validation() {
        let valid_consumer = Consumer {
            id: Some("test-id".into()),
            custom_id: None,
        };
        assert!(valid_consumer.is_valid());

        let invalid_consumer = Consumer {
            id: None,
            custom_id: None,
        };
        assert!(!invalid_consumer.is_valid());
    }
}
