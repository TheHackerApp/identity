use crate::{
    error::{Error, Result},
    Session,
};
use bytes::Bytes;
use chrono::Utc;
use redis::{aio::ConnectionManager, AsyncCommands};
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
                expiration as u64
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
