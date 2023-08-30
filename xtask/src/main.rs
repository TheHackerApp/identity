use clap::{Parser, Subcommand};
use common::logging;
use tracing::{debug, Level};

mod migrate;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    color_eyre::install()?;
    common::dotenv()?;

    let args = Args::parse();
    logging::init(args.log_level);

    debug!(?args);

    match args.command {
        Command::Migrate(args) => migrate::run(args).await,
    }
}

/// A collection of various development tasks
#[derive(Debug, Parser)]
#[command(author, version, about)]
pub struct Args {
    /// The default level to log at
    #[arg(short, long, default_value_t = Level::INFO, env = "LOG_LEVEL")]
    log_level: Level,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Manage database migrations
    Migrate(migrate::Args),
}
