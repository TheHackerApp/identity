use crate::util;
use database::{PgPool, Provider, User};
use eyre::{eyre, WrapErr};
use session::{AuthenticatedState, RegistrationNeededState, Session, SessionState};
use tracing::info;
use url::Url;

pub async fn run(args: Args) -> eyre::Result<()> {
    let cache = util::connect_to_cache(&args.cache_url).await?;
    let db = util::connect_to_database(&args.database_url).await?;

    // We can set fake values for the domain, secure, and signing key options since we're only
    // generating session tokens, not cookies.
    let manager = session::Manager::new(cache, "xtask", false, "xtask");

    let mut session = Session::default();
    session.state = match args.session_type {
        SessionType::Unauthenticated => SessionState::Unauthenticated,
        SessionType::RegistrationNeeded(opts) => {
            let provider = opts.retrieve_provider_slug(&db).await?;

            SessionState::RegistrationNeeded(RegistrationNeededState {
                provider,
                id: opts.id,
                email: opts.email,
                return_to: opts.return_to,
            })
        }
        SessionType::Authenticated(opts) => {
            let id = opts.retrieve_user_id(&db).await?;
            SessionState::Authenticated(AuthenticatedState { id })
        }
    };

    manager
        .save(&session)
        .await
        .wrap_err("failed to save session")?;

    let token = session
        .token(args.signing_key.as_bytes())
        .expect("session must have secret part");
    info!(%token, id = %session.id(), "generated session token");

    Ok(())
}

#[derive(clap::Args, Debug)]
pub struct Args {
    /// The Redis cache to store sessions in
    #[arg(long, env = "CACHE_URL")]
    cache_url: String,

    /// The database to run migrations on
    #[arg(short, long, env = "DATABASE_URL")]
    database_url: String,

    /// A secret to sign the session cookie with
    ///
    /// This should be a long, random string
    #[arg(long, env = "COOKIE_SIGNING_KEY")]
    signing_key: String,

    #[clap(subcommand)]
    session_type: SessionType,
}

#[derive(Debug, clap::Subcommand)]
#[clap(rename_all = "kebab-case")]
enum SessionType {
    /// Create an unauthenticated session
    #[command(alias("u"))]
    Unauthenticated,
    /// Create a session that needs to complete registration
    #[command(alias("rn"))]
    RegistrationNeeded(RegistrationNeededOptions),
    /// Creates an authenticated session
    #[command(alias("a"))]
    Authenticated(AuthenticatedOptions),
}

#[derive(clap::Args, Debug)]
struct RegistrationNeededOptions {
    /// The slug of the provider the user authenticated with
    #[arg(short, long)]
    provider: String,
    /// The user's ID according to the provider
    #[arg(short, long)]
    id: String,
    /// The user's primary email
    #[arg(short, long)]
    email: String,
    /// Where the user was redirected from
    #[arg(short, long)]
    return_to: Option<Url>,
}

impl RegistrationNeededOptions {
    /// Validate the provider exists and retrieve the it's slug
    async fn retrieve_provider_slug(&self, db: &PgPool) -> eyre::Result<String> {
        let provider = Provider::find(&self.provider, db)
            .await?
            .ok_or_else(|| eyre!("could not find provider"))?;

        Ok(provider.slug)
    }
}

#[derive(clap::Args, Debug)]
#[group(required = true, multiple = false)]
struct AuthenticatedOptions {
    /// The user's ID
    #[arg(short, long)]
    id: Option<i32>,
    /// The user's primary email
    #[arg(short, long)]
    email: Option<String>,
}

impl AuthenticatedOptions {
    /// Validate the user exists and retrieve their ID
    async fn retrieve_user_id(self, db: &PgPool) -> eyre::Result<i32> {
        let user = match (self.id, self.email) {
            (Some(id), None) => User::find(id, db).await?,
            (None, Some(email)) => User::find_by_primary_email(&email, db).await?,
            _ => unreachable!(),
        }
        .ok_or_else(|| eyre!("could not find user"))?;

        Ok(user.id)
    }
}
