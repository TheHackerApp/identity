use crate::Result;
use chrono::{DateTime, Utc};
use sqlx::{query, query_as, PgPool};
use tracing::instrument;

/// Maps a user to their authentication provider
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct Identity {
    /// The provider the identity corresponds to
    pub provider: String,
    /// The user the identity is linked to
    #[graphql(skip)]
    pub user_id: i32,
    /// The user's ID as given by the provider
    #[graphql(skip)]
    pub remote_id: String,
    /// The email associated with the identity
    pub email: String,
    /// When the identity was first created
    pub created_at: DateTime<Utc>,
    /// When the identity was last updated
    pub updated_at: DateTime<Utc>,
}

impl Identity {
    /// Get all the identities associated with a provider
    #[instrument(name = "Identity::for_user", skip(db))]
    pub async fn for_user(user_id: i32, db: &PgPool) -> Result<Vec<Identity>> {
        let identities = query_as!(
            Identity,
            "SELECT * FROM identities WHERE user_id = $1",
            user_id,
        )
        .fetch_all(db)
        .await?;
        Ok(identities)
    }

    /// Find an identity by it's provider and remote id
    #[instrument(name = "Identity::find_by_remote_id", skip(db))]
    pub async fn find_by_remote_id(
        provider: &str,
        remote_id: &str,
        db: &PgPool,
    ) -> Result<Option<Identity>> {
        let identity = query_as!(
            Identity,
            "SELECT * FROM identities WHERE provider = $1 AND remote_id = $2",
            provider,
            remote_id,
        )
        .fetch_optional(db)
        .await?;
        Ok(identity)
    }

    /// Link a user to a provider
    #[instrument(name = "Identity::link", skip(db))]
    pub async fn link(
        provider: &str,
        user_id: i32,
        remote_id: &str,
        email: &str,
        db: &PgPool,
    ) -> Result<Identity> {
        let identity = query_as!(
            Identity,
            r#"
            INSERT INTO identities (provider, user_id, remote_id, email) 
            VALUES ($1, $2, $3, $4) 
            RETURNING *
            "#,
            provider,
            user_id,
            remote_id,
            email,
        )
        .fetch_one(db)
        .await?;
        Ok(identity)
    }

    /// Update the email associated with an identity
    #[instrument(name = "Identity::update_email", skip(self, db), fields(%self.provider, %self.user_id))]
    pub async fn update_email(&mut self, email: String, db: &PgPool) -> Result<()> {
        query!(
            "UPDATE identities SET email = $3 WHERE provider = $1 AND user_id = $2",
            &self.provider,
            &self.user_id,
            &email,
        )
        .execute(db)
        .await?;

        self.email = email;

        Ok(())
    }

    /// Unlink a user from a provider
    #[instrument(name = "Identity::unlink", skip(db))]
    pub async fn unlink(provider: &str, user_id: i32, db: &PgPool) -> Result<()> {
        query!(
            "DELETE FROM identities WHERE provider = $1 AND user_id = $2",
            provider,
            user_id
        )
        .execute(db)
        .await?;
        Ok(())
    }
}
