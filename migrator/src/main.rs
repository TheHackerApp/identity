use clap::Parser;
use common::logging;
use eyre::WrapErr;
use tracing::{debug, Level};

#[tokio::main]
async fn main() -> eyre::Result<()> {
    color_eyre::install()?;
    common::dotenv()?;

    let args = Config::parse();
    logging::init(args.log_level);

    debug!(?args);

    let db = database::connect(&args.database_url).await?;
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
}
