use chrono;
use std::net;
use tracing::info;

use axum::{routing::post, Router};
use clap::Parser;
use tower_http::trace::TraceLayer;

use tracing_ext::{
    graphql_request_tracing_middleware, init_tracing, ExportTracesStdout, PropagateBaggage,
};

mod auth_handler;
mod cli;
mod errors;
mod validator;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    let opt = cli::ServerCli::parse();
    let port = opt.port;
    let export_traces_stdout = if opt.export_traces_stdout {
        ExportTracesStdout::Enable
    } else {
        ExportTracesStdout::Disable
    };
    let propagate_caller_baggage = if opt.propagate_caller_baggage {
        PropagateBaggage::Enable
    } else {
        PropagateBaggage::Disable
    };

    init_tracing(
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
        opt.otlp_endpoint.as_deref(),
        propagate_caller_baggage,
        export_traces_stdout,
    )?;

    let mut router = Router::new()
        .route("/validate-request", post(auth_handler::validate_request))
        .layer(axum::middleware::from_fn(
            graphql_request_tracing_middleware,
        ));
    if let Some(_) = opt.otlp_endpoint {
        router = router.layer(TraceLayer::new_for_http());
    }

    let host = net::IpAddr::V6(net::Ipv6Addr::UNSPECIFIED);

    let address = (host, port);
    let listener = tokio::net::TcpListener::bind(address).await?;

    let server = axum::serve(listener, router.into_make_service());
    info!("Server started on port {}", port);
    server
        .with_graceful_shutdown(axum_ext::shutdown_signal_with_handler(|| async move {
            info!("Received shutdown signal at {}", chrono::Local::now());
        }))
        .await?;
    info!("Server shutdown at {}", chrono::Local::now());
    Ok(())
}
