use axum::Server;
use clap::Parser;
use eyre::{eyre, WrapErr};
use globset::{Glob, GlobSet};
use logging::OpenTelemetryProtocol;
use redis::aio::ConnectionManager as RedisConnectionManager;
use std::net::SocketAddr;
use tokio::signal;
use tracing::{info, Level};
use url::Url;

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

    let db = database::connect(&config.database_url).await?;
    let cache = connect_to_cache(&config.cache_url).await?;

    let allowed_redirect_domains = config
        .allowed_redirect_domains_matcher()
        .wrap_err("invalid allowed redirect domains")?;
    let router = identity::router(
        config.api_url,
        cache,
        db,
        config.frontend_url,
        allowed_redirect_domains,
        config.domain_suffix,
        config.admin_domains,
        config.user_domains,
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

    /// The domain suffix where non-custom domains are hosted
    #[arg(
        long,
        default_value = ".thehacker.app",
        value_parser = valid_domain_suffix,
        env = "DOMAIN_SUFFIX",
    )]
    domain_suffix: String,

    /// A comma-separated list of domains which require the admin scope
    #[arg(long, value_delimiter = ',', env = "ADMIN_DOMAINS")]
    admin_domains: Vec<String>,

    /// A comma-separated list of domains which require the user scope
    #[arg(long, value_delimiter = ',', env = "USER_DOMAINS")]
    user_domains: Vec<String>,

    /// A comma-separated list of domains that the OAuth flow is allowed to return to
    ///
    /// Allows globs in individual domains. Also automatically includes any registered custom domains
    #[arg(long, value_delimiter = ',', env = "ALLOWED_REDIRECT_DOMAINS")]
    allowed_redirect_domains: Vec<String>,

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
        value_parser = opentelemetry_protocol_parser,
        env = "OTEL_EXPORTER_OTLP_PROTOCOL",
    )]
    opentelemetry_protocol: OpenTelemetryProtocol,
}

impl Config {
    /// Construct a matcher for the allowed redirect domains
    fn allowed_redirect_domains_matcher(&self) -> Result<GlobSet, globset::Error> {
        let mut set = GlobSet::builder();

        for glob in &self.allowed_redirect_domains {
            let glob = Glob::new(glob)?;
            set.add(glob);
        }
        set.build()
    }
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

/// Parse the domain suffix from a command line argument
fn valid_domain_suffix(s: &str) -> eyre::Result<String> {
    if !s.starts_with('.') {
        return Err(eyre!("domain suffix must start with a '.'"));
    }
    if s.chars().filter(|c| *c == '.').count() < 2 {
        return Err(eyre!("domain suffix must contain at least a base and TLD"));
    }

    Ok(s.to_owned())
}

/// Parse the OpenTelemetry protocol from a command line argument
fn opentelemetry_protocol_parser(raw: &str) -> eyre::Result<OpenTelemetryProtocol> {
    match raw.to_lowercase().as_str() {
        "grpc" => Ok(OpenTelemetryProtocol::Grpc),
        "http" | "http/protobuf" => Ok(OpenTelemetryProtocol::HttpBinary),
        _ => Err(eyre!(
            "invalid exporter protocol, must be one of: 'grpc' or 'http/protobuf'"
        )),
    }
}
