use eyre::WrapErr;
use sqlx::{
    postgres::{PgConnectOptions, PgPoolOptions},
    ConnectOptions,
};
use std::{
    fmt::{Debug, Display, Formatter},
    str::FromStr,
    sync::Arc,
    time::Duration,
};
use tracing::{info, instrument, log::LevelFilter};

mod identity;
mod organizer;
mod participant;
mod provider;
mod types;
mod user;

pub use identity::Identity;
pub use organizer::Organizer;
pub use participant::Participant;
pub use provider::{Provider, ProviderConfiguration};
pub use sqlx::PgPool;
pub use types::Json;
pub use user::User;

pub(crate) type Result<T, E = Error> = std::result::Result<T, E>;

/// Connect to the database and ensure it works
#[instrument(skip_all)]
pub async fn connect(url: &str) -> eyre::Result<PgPool> {
    let options = PgConnectOptions::from_str(url)
        .wrap_err("invalid database url format")?
        .log_statements(LevelFilter::Info)
        .log_slow_statements(LevelFilter::Warn, Duration::from_secs(5));

    let db = PgPoolOptions::new()
        .acquire_timeout(Duration::from_secs(10))
        .connect_with(options)
        .await
        .wrap_err("failed to connect to the database")?;

    info!("database connected");
    Ok(db)
}

/// Represents the different way the database can fail
#[derive(Clone)]
pub struct Error(Arc<sqlx::Error>);

impl Error {
    /// Returns whether the error kind is a violation of a unique/primary key constraint.
    pub fn is_unique_violation(&self) -> bool {
        match self.0.as_ref() {
            sqlx::Error::Database(e) => e.is_unique_violation(),
            _ => false,
        }
    }
}

impl Debug for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.0, f)
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.0.source()
    }
}

#[cfg(feature = "graphql")]
impl async_graphql::ErrorExtensions for Error {
    fn extend(&self) -> async_graphql::Error {
        use std::error::Error as _;

        match self.source() {
            Some(e) => tracing::error!(error = %self.0, source = %e, "unexpected database error"),
            None => tracing::error!(error = %self.0, "unexpected database error"),
        }

        async_graphql::Error::new("internal server error")
    }
}

impl From<sqlx::Error> for Error {
    fn from(error: sqlx::Error) -> Self {
        Self(Arc::new(error))
    }
}
