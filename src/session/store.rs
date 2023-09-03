use super::Session;
use bytes::Bytes;
use chrono::Utc;
use redis::{aio::ConnectionManager, AsyncCommands, RedisError};
use std::fmt::{Display, Formatter};
use tracing::instrument;

/// The session storage backend
#[derive(Clone)]
pub(crate) struct Store {
    manager: ConnectionManager,
}

impl Store {
    /// Create a new storage backend
    pub fn new(manager: ConnectionManager) -> Self {
        Self { manager }
    }

    /// Load a session
    #[instrument(name = "Store::load", skip(self))]
    pub async fn load(&self, id: &str) -> Result<Option<Session>> {
        let mut conn = self.manager.clone();
        let raw = conn
            .get::<_, Option<Bytes>>(format!("identity:session:{id}"))
            .await?;

        raw.map(|bytes| {
            serde_json::from_slice(&bytes).map_err(|e| Error::Json {
                source: e,
                content: bytes,
            })
        })
        .transpose()
    }

    /// Persist a session
    #[instrument(name = "Store::save", skip_all, fields(id = %session.id))]
    pub async fn save(&self, session: &Session) -> Result<()> {
        let value = serde_json::to_vec(session).expect("session must serialize");

        let expiration = {
            let expiration = (session.expiry - Utc::now()).num_seconds();
            if expiration > 0 {
                expiration as usize
            } else {
                0
            }
        };

        let mut conn = self.manager.clone();
        conn.set_ex(
            format!("identity:session:{}", session.id),
            value,
            expiration,
        )
        .await?;

        Ok(())
    }
}

type Result<T, E = Error> = std::result::Result<T, E>;

/// Errors that can occur when storing/retrieving sessions
#[derive(Debug)]
pub(crate) enum Error {
    /// Error while interacting with Redis
    Redis(RedisError),
    /// Unable to deserialize session
    Json {
        source: serde_json::Error,
        content: Bytes,
    },
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Redis(_) => write!(f, "error while interacting with redis"),
            Self::Json { content, .. } => {
                // content _should_ always be UTF8
                let content = String::from_utf8_lossy(content);
                write!(f, "failed to deserialize session: {content}")
            }
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Redis(e) => Some(e),
            Self::Json { source, .. } => Some(source),
        }
    }
}

impl From<RedisError> for Error {
    fn from(error: RedisError) -> Self {
        Self::Redis(error)
    }
}
