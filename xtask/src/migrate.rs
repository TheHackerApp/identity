use crate::util;
use eyre::WrapErr;
use sqlx::migrate::Migrator;
use std::path::PathBuf;

pub async fn run(args: Args) -> eyre::Result<()> {
    let migrator = Migrator::new(&*args.source)
        .await
        .wrap_err("failed to load migrations")?;

    let db = util::connect_to_database(&args.database_url).await?;

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
