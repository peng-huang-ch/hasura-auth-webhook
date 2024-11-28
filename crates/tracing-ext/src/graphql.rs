use axum_core::body::Body;
use opentelemetry::global::get_text_map_propagator;
use opentelemetry_http::HeaderInjector;

use crate::{global_tracer, SpanVisibility, TraceableHttpResponse};

pub async fn graphql_request_tracing_middleware(
    request: http::Request<Body>,
    next: axum::middleware::Next,
) -> axum::response::Result<axum::response::Response> {
    let traceable = global_tracer()
        .in_span_async_with_parent_context(
            "request",
            "request",
            SpanVisibility::User,
            &request.headers().clone(),
            || {
                Box::pin(async move {
                    let mut response = next.run(request).await;

                    get_text_map_propagator(|propagator| {
                        propagator.inject(&mut HeaderInjector(response.headers_mut()))
                    });
                    TraceableHttpResponse::new(response, "")
                })
            },
        )
        .await;
    Ok(traceable.response)
}
