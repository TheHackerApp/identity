use clap::Parser;
use common::logging::{OpenTelemetry, OpenTelemetryProtocol};
use eyre::WrapErr;
use tracing::{debug, Level};

#[tokio::main]
async fn main() -> eyre::Result<()> {
    color_eyre::install()?;
    common::dotenv()?;

    let config = Config::parse();
    common::logging::init(
        config.log_level,
        OpenTelemetry::new(
            config.opentelemetry_endpoint.as_deref(),
            config.opentelemetry_protocol,
        ),
    )?;

    debug!(?config);

    let db = database::connect(&config.database_url).await?;
    migrator::apply(&db)
        .await
        .wrap_err("failed to apply migrations")
}

/// Run schema migrations on the database
#[derive(Debug, Parser)]
#[command(author, version, about)]
struct Config {
    /// The default level to log at
    ///
    /// More specific log targets can be set using the `RUST_LOG` environment variable. They must be
    /// formatted as tracing directives: https://docs.rs/tracing-subscriber/latest/tracing_subscriber/filter/struct.EnvFilter.html#directives
    #[arg(short, long, default_value_t = Level::INFO, env = "LOG_LEVEL")]
    log_level: Level,

    /// The database to run migrations on
    #[arg(short, long, env = "DATABASE_URL")]
    database_url: String,

    /// The OpenTelemetry endpoint to send traces to
    #[arg(long, env = "OTEL_EXPORTER_OTLP_ENDPOINT")]
    opentelemetry_endpoint: Option<String>,

    /// The protocol to use when exporting OpenTelemetry traces
    #[arg(
    long,
    default_value = "grpc",
    value_parser = common::logging::opentelemetry_protocol_parser,
    env = "OTEL_EXPORTER_OTLP_PROTOCOL",
    )]
    opentelemetry_protocol: OpenTelemetryProtocol,
}
