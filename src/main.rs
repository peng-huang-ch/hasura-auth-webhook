use chrono;
use std::net;
use tracing::{info, level_filters::LevelFilter};
use tracing_subscriber::EnvFilter;

use axum::{routing::post, Router};
use clap::Parser;
use tower_http::trace::TraceLayer;

mod auth_handler;
mod cli;
mod errors;
mod shutdown;
mod validator;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    dotenvy::dotenv().ok();
    let opt = cli::ServerCli::parse();
    let port = opt.port;

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::DEBUG.into())
                .from_env_lossy(),
        )
        .without_time()
        .init();

    let app = Router::new()
        .route("/validate-request", post(auth_handler::validate_request))
        .layer(TraceLayer::new_for_http());
    let host = net::IpAddr::V6(net::Ipv6Addr::UNSPECIFIED);

    let address = (host, port);
    let listener = tokio::net::TcpListener::bind(address).await?;

    let server = axum::serve(listener, app.into_make_service());
    info!("Server started on port {}", port);
    server
        .with_graceful_shutdown(shutdown::shutdown_signal())
        .await?;
    info!("Server shutdown at {}", chrono::Local::now());
    Ok(())
}
