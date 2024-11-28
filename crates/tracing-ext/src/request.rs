//! Functions to provide the appropriate headers when making a HTTP request.

// Extract the headers required to propagate the trace context across services from the current
// context.
pub fn get_trace_headers() -> http::HeaderMap {
    let mut header_map = http::HeaderMap::new();
    let mut header_injector = opentelemetry_http::HeaderInjector(&mut header_map);
    opentelemetry::global::get_text_map_propagator(|propagator| {
        propagator.inject(&mut header_injector);
    });
    header_map
}
