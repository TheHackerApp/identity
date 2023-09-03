use axum::Server;
use clap::Parser;
use common::logging::{OpenTelemetry, OpenTelemetryProtocol};
use eyre::WrapErr;
use redis::aio::ConnectionManager as RedisConnectionManager;
use std::net::SocketAddr;
use tokio::signal;
use tracing::{info, Level};
use url::Url;

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

    let db = database::connect(&config.database_url).await?;
    let cache = connect_to_cache(&config.cache_url).await?;

    let router = identity::router(
        config.api_url,
        cache,
        db,
        config.frontend_url,
        &config.cookie_signing_key,
    );

    info!(address = %config.address, "listening and ready to handle requests");
    Server::bind(&config.address)
        .serve(router.into_make_service())
        .with_graceful_shutdown(shutdown())
        .await
        .wrap_err("failed to start server")?;

    Ok(())
}

/// Connect to the specified cache instance
async fn connect_to_cache(url: &str) -> eyre::Result<RedisConnectionManager> {
    let client = redis::Client::open(url).wrap_err("invalid cache URL format")?;
    let manager = client
        .get_tokio_connection_manager()
        .await
        .wrap_err("failed to connect to the cache")?;
    Ok(manager)
}

/// Setup hyper graceful shutdown for SIGINT (ctrl+c) and SIGTERM
async fn shutdown() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install ctrl+c handler")
    };
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install sigterm handler")
            .recv()
            .await
    };

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    info!("server successfully shutdown");
    info!("goodbye! o/");
}

/// The authentication and authorization service for the hacker app
#[derive(Debug, Parser)]
#[command(author, version, about)]
struct Config {
    /// The address for the server to listen on
    #[arg(long, default_value = "127.0.0.1:4243", env = "ADDRESS")]
    address: SocketAddr,

    /// The database to run migrations on
    #[arg(long, env = "DATABASE_URL")]
    database_url: String,

    /// The Redis cache to store sessions in
    #[arg(long, env = "CACHE_URL")]
    cache_url: String,

    /// The default level to log at
    #[arg(long, default_value_t = Level::INFO, env = "LOG_LEVEL")]
    log_level: Level,

    /// The publicly accessible URL for the API
    #[arg(long, env = "API_URL")]
    api_url: Url,

    /// The publicly accessible URL for the frontend
    #[arg(long, env = "FRONTEND_URL")]
    frontend_url: Url,

    /// A secret to sign the session cookie with
    ///
    /// This should be a long, random string
    #[arg(long, env = "COOKIE_SIGNING_KEY")]
    cookie_signing_key: String,

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
