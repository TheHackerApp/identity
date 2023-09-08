use clap::Parser;
use eyre::{eyre, WrapErr};
use logging::OpenTelemetryProtocol;
use tracing::{debug, Level};

#[tokio::main]
async fn main() -> eyre::Result<()> {
    color_eyre::install()?;
    dotenv()?;

    let config = Config::parse();

    let mut logging = logging::config().default_directive(config.log_level);
    if let Some(endpoint) = &config.opentelemetry_endpoint {
        logging = logging.opentelemetry(config.opentelemetry_protocol, endpoint);
    }
    logging.init()?;

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
    value_parser = opentelemetry_protocol_parser,
    env = "OTEL_EXPORTER_OTLP_PROTOCOL",
    )]
    opentelemetry_protocol: OpenTelemetryProtocol,
}

/// Load environment variables from a .env file, if it exists.
fn dotenv() -> eyre::Result<()> {
    if let Err(error) = dotenvy::dotenv() {
        if !error.not_found() {
            return Err(error).wrap_err("failed to load .env");
        }
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
