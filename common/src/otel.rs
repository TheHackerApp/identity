use opentelemetry::{
    sdk::{
        resource::{
            EnvResourceDetector, OsResourceDetector, ProcessResourceDetector,
            SdkProvidedResourceDetector, TelemetryResourceDetector,
        },
        trace::{self, Tracer},
        Resource,
    },
    trace::TraceError,
};
use opentelemetry_otlp::{Protocol, SpanExporterBuilder, WithExportConfig};
use std::time::Duration;

/// Create a new tracing pipeline
pub fn tracer(protocol: Protocol, url: &str) -> Result<Tracer, TraceError> {
    let exporter = exporter(protocol, url);

    opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(exporter)
        .with_trace_config(trace_config())
        .install_batch(opentelemetry::runtime::Tokio)
}

/// Create a new span exporter
fn exporter(protocol: Protocol, url: &str) -> SpanExporterBuilder {
    match protocol {
        Protocol::Grpc => opentelemetry_otlp::new_exporter()
            .tonic()
            .with_env()
            .with_endpoint(url)
            .into(),
        Protocol::HttpBinary => opentelemetry_otlp::new_exporter()
            .http()
            .with_env()
            .with_endpoint(url)
            .into(),
    }
}

/// Create a new tracing configuration
fn trace_config() -> trace::Config {
    trace::config().with_resource(resource_detectors())
}

/// Setup resource detectors to populate environment
fn resource_detectors() -> Resource {
    Resource::from_detectors(
        Duration::from_secs(5),
        vec![
            Box::new(SdkProvidedResourceDetector),
            Box::<EnvResourceDetector>::default(),
            Box::new(OsResourceDetector),
            Box::new(ProcessResourceDetector),
            Box::new(TelemetryResourceDetector),
        ],
    )
}
