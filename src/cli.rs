use clap::Parser;

#[derive(Debug, Parser)]
pub struct ServerCli {
    /// The OpenTelemetry collector endpoint.
    #[arg(
        long,
        value_name = "URL",
        value_name = "OTLP_ENDPOINT",
        env = "OTLP_ENDPOINT"
    )]
    pub otlp_endpoint: Option<String>,

    /// Log traces to stdout.
    #[arg(
        long,
        value_name = "EXPORT_TRACES_STDOUT",
        env = "EXPORT_TRACES_STDOUT"
    )]
    pub export_traces_stdout: bool,

    /// Port.
    #[arg(long, value_name = "PORT", env = "PORT")]
    pub port: u16,

    /// Kong URL.
    #[arg(long, value_name = "KONG_URL", env = "KONG_URL")]
    pub kong_url: String,
}
