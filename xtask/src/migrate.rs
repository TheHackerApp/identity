use eyre::WrapErr;
use sqlx::{
    migrate::Migrator,
    postgres::{PgConnectOptions, PgPool},
    ConnectOptions,
};
use std::{path::PathBuf, str::FromStr};
use tracing::log::LevelFilter;

pub async fn run(args: Args) -> eyre::Result<()> {
    let migrator = Migrator::new(&*args.source)
        .await
        .wrap_err("failed to load migrations")?;

    let options = PgConnectOptions::from_str(&args.database_url)
        .wrap_err("invalid database URL format")?
        .log_statements(LevelFilter::Debug);
    let db = PgPool::connect_with(options)
        .await
        .wrap_err("failed to connect to the database")?;

    match args.command {
        Command::Add { name } => migrator::add(&args.source, &name.join("_"))?,
        Command::Info => migrator::info(&migrator, &db).await?,
        Command::Apply => migrator::apply(&migrator, &db).await?,
        Command::Revert { target } => migrator::undo(&migrator, &db, target).await?,
    }

    Ok(())
}

#[derive(clap::Args, Debug)]
pub struct Args {
    /// The database to run migrations on
    #[arg(short, long, env = "DATABASE_URL")]
    database_url: String,

    /// The migrations source
    #[arg(short, long, default_value = "./migrations")]
    source: PathBuf,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, clap::Subcommand)]
pub enum Command {
    /// Create a new migration
    Add {
        /// The name of the migration
        #[arg(required = true)]
        name: Vec<String>,
    },
    /// List all available migrations
    Info,
    /// Apply all pending migrations
    Apply,
    /// Revert migrations
    ///
    /// If no target is provided, the most recent migration is reverted.
    Revert {
        /// The version to revert back to
        target: Option<i64>,
    },
}
