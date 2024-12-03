use opentelemetry::{
    baggage::BaggageExt, global, propagation::composite::TextMapCompositePropagator,
    propagation::TextMapPropagator, trace::Span, trace::TraceError, KeyValue,
};
pub use opentelemetry_contrib::trace::propagator::trace_context_response::TraceContextResponsePropagator;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::propagation::{BaggagePropagator, TraceContextPropagator};
use opentelemetry_sdk::trace::{SpanProcessor, TracerProvider};
use opentelemetry_semantic_conventions as semcov;
use tracing_subscriber::{
    filter::LevelFilter,
    fmt,
    layer::SubscriberExt,    // for `with`
    util::SubscriberInitExt, // for `init`
    EnvFilter,
};

/**
 * This module provides functionality for OpenTelemetry tracing setup and configuration.
 * It includes support for:
 * - OTLP exporter configuration
 * - Baggage propagation
 * - Stdout trace export
 * - Multiple propagators (TraceContext, Zipkin, Baggage, TraceContextResponse)
 * - Resource attributes for service identification
 *
 * The implementation is adapted from the opentelemetry-rust-contrib project.
 * See https://docs.rs/opentelemetry-otlp/latest/opentelemetry_otlp/ for more details.
 *
 * The original code is licensed under the Apache License, Version 2.0,
 * (the "License"); you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 * http://www.apache.org/licenses/LICENSE-2.0
 */

/// Controls whether baggage propagation is enabled or disabled
///
/// Baggage allows passing key-value pairs across service boundaries through
/// propagation headers.
#[derive(Debug, Copy, Clone)]
pub enum PropagateBaggage {
    /// Enable baggage propagation from upstream services
    Enable,
    /// Disable baggage propagation from upstream services
    Disable,
}

/// Controls whether traces are exported to stdout for debugging
#[derive(Debug, Copy, Clone)]
pub enum ExportTracesStdout {
    /// Enable trace export to stdout
    Enable,
    /// Disable trace export to stdout  
    Disable,
}

/// A span processor that adds baggage key-value pairs as span attributes
///
/// The BaggageSpanProcessor extracts baggage from the Context and adds each
/// key-value pair as an attribute on new spans. This allows baggage data to
/// be visible in traces.
///
/// Baggage is OpenTelemetry's mechanism for propagating key-value pairs across
/// service boundaries. The data is passed in request headers using the W3C
/// Baggage format (https://w3c.github.io/baggage/).
///
/// Baggage can be added to the current context using Context::current_with_baggage().
#[derive(Debug)]
pub struct BaggageSpanProcessor();

impl SpanProcessor for BaggageSpanProcessor {
    fn on_start(&self, span: &mut opentelemetry_sdk::trace::Span, cx: &opentelemetry::Context) {
        for (key, (value, _)) in cx.baggage() {
            span.set_attribute(KeyValue::new(key.to_string(), value.to_string()));
        }
    }

    fn on_end(&self, _span: opentelemetry_sdk::export::trace::SpanData) {}

    fn force_flush(&self) -> opentelemetry::trace::TraceResult<()> {
        Ok(())
    }

    fn shutdown(&self) -> opentelemetry::trace::TraceResult<()> {
        Ok(())
    }
}

/// A propagator wrapper that only allows context injection, not extraction
///
/// This propagator wraps another TextMapPropagator but only implements the
/// inject_context() functionality. The extract_with_context() method is a no-op
/// that just returns the original context.
///
/// This is useful when you want to propagate context to downstream services
/// but not accept context from upstream services.
#[derive(Debug)]
pub struct InjectOnlyTextMapPropagator<T>(T);

impl<T: TextMapPropagator> TextMapPropagator for InjectOnlyTextMapPropagator<T> {
    fn inject_context(
        &self,
        cx: &opentelemetry::Context,
        injector: &mut dyn opentelemetry::propagation::Injector,
    ) {
        self.0.inject_context(cx, injector);
    }

    fn extract_with_context(
        &self,
        cx: &opentelemetry::Context,
        _extractor: &dyn opentelemetry::propagation::Extractor,
    ) -> opentelemetry::Context {
        cx.clone()
    }

    fn fields(&self) -> opentelemetry::propagation::text_map_propagator::FieldIter<'_> {
        self.0.fields()
    }
}

/// Initialize OpenTelemetry tracing with the specified configuration
///
/// This sets up:
/// - Global tracer provider
/// - OTLP exporter (if endpoint provided)
/// - Stdout exporter (if enabled)
/// - Context propagation via standard headers
/// - Baggage propagation (configurable)
/// - Resource attributes for service identification
///
/// # Arguments
///
/// * `service_name` - Name of the service for resource attribution
/// * `service_version` - Version of the service for resource attribution  
/// * `endpoint` - Optional OTLP endpoint URL (e.g. "http://localhost:4317")
/// * `propagate_caller_baggage` - Whether to propagate baggage from upstream
/// * `enable_stdout_export` - Whether to export traces to stdout
///
/// # Returns
///
/// Returns `Ok(())` if setup succeeds, or a `TraceError` if initialization fails
pub fn init_tracing(
    service_name: &'static str,
    service_version: &'static str,
    endpoint: Option<&str>,
    propagate_caller_baggage: PropagateBaggage,
    enable_stdout_export: ExportTracesStdout,
) -> Result<(), TraceError> {
    // Install global collector configured based on RUST_LOG env var.
    let env_filter = EnvFilter::from_default_env().add_directive(LevelFilter::INFO.into());

    // Create a `tracing` layer to emit spans as structured logs to stdout
    let std_layer = fmt::layer().with_writer(std::io::stderr);
    let subscriber = tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt::layer().without_time())
        .with(std_layer);
    subscriber.init();

    // Initialize the tracer provider
    if let Some(endpoint) = endpoint {
        init_tracer_provider(
            service_name,
            service_version,
            endpoint,
            propagate_caller_baggage,
            enable_stdout_export,
        )?;
    }
    Ok(())
}

/// Initialize the OpenTelemetry tracer provider with the specified configuration
///
/// This sets up the tracer provider with:
/// - OTLP exporter
/// - Baggage processor
/// - Resource attributes
/// - Optional stdout exporter
///
/// # Arguments
///
/// * `service_name` - Name of the service for resource attribution
/// * `service_version` - Version of the service for resource attribution
/// * `endpoint` - OTLP endpoint URL (e.g. "http://localhost:4317")
/// * `propagate_caller_baggage` - Whether to propagate baggage from upstream
/// * `enable_stdout_export` - Whether to export traces to stdout
///
/// # Returns
///
/// Returns `Ok(())` if setup succeeds, or a `TraceError` if initialization fails
pub fn init_tracer_provider(
    service_name: &'static str,
    service_version: &'static str,
    endpoint: &str,
    propagate_caller_baggage: PropagateBaggage,
    enable_stdout_export: ExportTracesStdout,
) -> Result<(), TraceError> {
    let baggage_propagator: Box<dyn TextMapPropagator + Send + Sync> =
        match propagate_caller_baggage {
            PropagateBaggage::Enable => Box::new(BaggagePropagator::new()),
            PropagateBaggage::Disable => {
                Box::new(InjectOnlyTextMapPropagator(BaggagePropagator::new()))
            }
        };
    global::set_text_map_propagator(TextMapCompositePropagator::new(vec![
        Box::new(TraceContextPropagator::new()),
        Box::new(opentelemetry_zipkin::Propagator::new()),
        baggage_propagator,
        Box::new(TraceContextResponsePropagator::new()),
    ]));

    let resource_entries = vec![
        KeyValue::new(semcov::resource::SERVICE_NAME, service_name),
        KeyValue::new(semcov::resource::SERVICE_VERSION, service_version),
    ];

    let otlp_exporter_builder = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint(endpoint);

    let otlp_exporter = otlp_exporter_builder.build()?;

    let mut tracer_provider = TracerProvider::builder()
        .with_resource(opentelemetry_sdk::Resource::new(resource_entries))
        .with_batch_exporter(otlp_exporter, opentelemetry_sdk::runtime::Tokio)
        .with_span_processor(BaggageSpanProcessor());

    if let ExportTracesStdout::Enable = enable_stdout_export {
        let stdout_exporter = opentelemetry_stdout::SpanExporter::default();
        tracer_provider = tracer_provider.with_simple_exporter(stdout_exporter);
    }
    let tracer_provider = tracer_provider.build();

    // Set the global tracer provider so everyone gets this setup.
    global::set_tracer_provider(tracer_provider.clone());
    Ok(())
}

/// Shutdown the global tracer provider
///
/// This ensures any pending spans are exported before the program exits.
pub fn shutdown_tracer() {
    global::shutdown_tracer_provider();
}
