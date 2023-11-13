use clap::{Parser, Subcommand};
use eyre::WrapErr;
use tracing::{debug, Level};

mod export_schema;
mod migrate;
mod sessions;
mod util;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    color_eyre::install()?;
    dotenv()?;

    let args = Args::parse();
    logging::config().default_directive(args.log_level).init()?;

    debug!(?args);

    match args.command {
        Command::ExportSchema(args) => export_schema::run(args),
        Command::Migrate(args) => migrate::run(args).await,
        Command::Sessions(args) => sessions::run(args).await,
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
    /// Export the GraphQL schema to a file
    ExportSchema(export_schema::Args),
    /// Manage database migrations
    Migrate(migrate::Args),
    /// Generate sessions with custom attributes
    ///
    /// All session types, except for OAuth, can be created. An OAuth session cannot created due to
    /// its integration with 3rd-parties.
    Sessions(sessions::Args),
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
