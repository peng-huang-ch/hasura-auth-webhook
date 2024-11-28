mod shutdown;

// re-export things from OpenTelemetry to avoid library users importing their own version and
// risking mismatches and multiple globals
pub use shutdown::{shutdown_signal, shutdown_signal_with_handler};
