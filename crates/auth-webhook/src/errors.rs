// https://github.com/LemmyNet/lemmy/blob/main/crates/utils/src/error.rs#L73
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;
use std::fmt::{Debug, Display};
use tracing_error::SpanTrace;

#[allow(dead_code)]
pub type SrvResult<T> = Result<T, SrvError>;

// https://docs.rs/tracing-error/latest/tracing_error/
#[allow(dead_code)]
#[derive(Debug, thiserror::Error)]
pub enum SrvErrorKind {
    #[error("the data for key {0} is not found")]
    NotFound(String),

    #[error("{0}")]
    BadRequest(String),

    #[error("{0}")]
    Unauthorized(String),

    #[error("{1}")]
    Custom(StatusCode, String),

    #[error("{0}")]
    Any(#[from] anyhow::Error),

    #[error("reqwest error. {:?}", .0)]
    ReqwestError(#[from] reqwest::Error),
}

#[derive(Debug)]
pub struct SrvError {
    pub error_kind: SrvErrorKind,
    pub inner: anyhow::Error,
    pub context: SpanTrace,
}

impl Display for SrvError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: ", &self.error_kind)?;
        // print anyhow including trace
        // https://docs.rs/anyhow/latest/anyhow/struct.Error.html#display-representations
        // this will print the anyhow trace (only if it exists)
        // and if RUST_BACKTRACE=1, also a full backtrace
        writeln!(f, "{:?}", self.inner)?;
        // writeln!(f, "source {:?}", self.inner.backtrace())?;
        // print the tracing span trace
        std::fmt::Display::fmt(&self.context, f)
    }
}

impl<T> From<T> for SrvError
where
    T: Into<SrvErrorKind>,
{
    fn from(t: T) -> Self {
        let into = t.into();
        SrvError {
            inner: anyhow::anyhow!("{:?}", &into),
            error_kind: into,
            context: SpanTrace::capture(),
        }
    }
}

impl IntoResponse for SrvError {
    fn into_response(self) -> Response {
        let status_code = match &self.error_kind {
            SrvErrorKind::Custom(code, _) => code.clone(),
            SrvErrorKind::NotFound(_) => StatusCode::NOT_FOUND,
            SrvErrorKind::Any(_) => StatusCode::INTERNAL_SERVER_ERROR,
            SrvErrorKind::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            SrvErrorKind::BadRequest(_) => StatusCode::BAD_REQUEST,
            SrvErrorKind::ReqwestError(e) => StatusCode::from_u16(
                e.status()
                    .map(|f| f.as_u16())
                    .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR.as_u16()),
            )
            .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            // _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        let error_response = axum::Json(json!({
            "success": false,
            "code": status_code.as_u16(),
            "error": status_code.canonical_reason().unwrap_or("Unknown").to_string(),
            "message": self.error_kind.to_string(),
        }))
        .into_response();
        (status_code, error_response).into_response()
    }
}
