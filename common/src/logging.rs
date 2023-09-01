use crate::otel;
use eyre::{eyre, WrapErr};
use tracing::Level;
use tracing_error::ErrorLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Registry};

pub use opentelemetry_otlp::Protocol as OpenTelemetryProtocol;

/// General OpenTelemetry exporter configuration
#[derive(Debug)]
pub struct OpenTelemetry<'o> {
    url: &'o str,
    protocol: OpenTelemetryProtocol,
}

impl<'o> OpenTelemetry<'o> {
    /// Create a new OpenTelemetry configuration
    pub fn new(url: Option<&'o str>, protocol: OpenTelemetryProtocol) -> Option<Self> {
        url.map(|url| Self { url, protocol })
    }
}

/// Setup logging and error reporting
pub fn init(default_level: Level, opentelemetry: Option<OpenTelemetry<'_>>) -> eyre::Result<()> {
    let debug = cfg!(debug_assertions);

    let registry = Registry::default()
        .with(
            EnvFilter::builder()
                .with_default_directive(default_level.into())
                .from_env_lossy(),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_file(debug)
                .with_line_number(debug)
                .with_target(true),
        )
        .with(ErrorLayer::default());

    if let Some(opentelemetry) = opentelemetry {
        let tracer = otel::tracer(opentelemetry.protocol, opentelemetry.url)
            .wrap_err("failed to initialize opentelemetry tracer")?;

        let opentelemetry = tracing_opentelemetry::layer()
            .with_location(true)
            .with_tracked_inactivity(true)
            .with_exception_field_propagation(true)
            .with_exception_fields(true)
            .with_tracer(tracer);

        registry.with(opentelemetry).init();
    } else {
        registry.init();
    }

    Ok(())
}

/// Parse the OpenTelemetry protocol from a command line argument
pub fn opentelemetry_protocol_parser(raw: &str) -> eyre::Result<OpenTelemetryProtocol> {
    match raw.to_lowercase().as_str() {
        "grpc" => Ok(OpenTelemetryProtocol::Grpc),
        "http" | "http/protobuf" => Ok(OpenTelemetryProtocol::HttpBinary),
        _ => Err(eyre!(
            "invalid exporter protocol, must be one of: 'grpc' or 'http/protobuf'"
        )),
    }
}
