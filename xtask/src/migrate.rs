pub async fn run(args: Args) -> eyre::Result<()> {
    let db = common::database::connect(&args.database_url).await?;

    match args.command {
        Command::Add { name } => migrator::add(name.join("_"))?,
        Command::Info => migrator::info(&db).await?,
        Command::Apply => migrator::apply(&db).await?,
        Command::Revert { target } => migrator::undo(&db, target).await?,
    }

    Ok(())
}

#[derive(clap::Args, Debug)]
pub struct Args {
    /// The database to run migrations on
    #[arg(short, long, env = "DATABASE_URL")]
    database_url: String,

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
