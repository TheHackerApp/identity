use crate::types::Json;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{query, query_as, PgPool};
use std::fmt::{Debug, Formatter};
use tracing::instrument;

/// Configuration for an authentication provider
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct Provider {
    /// A unique identifier for the provider
    pub slug: String,
    /// Whether the provider can be used for authentication
    pub enabled: bool,
    /// The display name
    pub name: String,
    /// The URL for the provider's icon
    pub icon: String,
    /// Provider-specific configuration, i.e. implementation kind, OIDC URLs, scopes, etc
    pub config: Json<ProviderConfiguration>,
    /// When the provider was created
    pub created_at: DateTime<Utc>,
    /// WHen the provider was last updated
    pub updated_at: DateTime<Utc>,
}

/// The provider-specific configuration
#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind")]
pub enum ProviderConfiguration {
    /// An OpenID connect provider
    OpenIDConnect {
        /// The client ID
        client_id: String,
        /// The client secret
        client_secret: String,
        /// The URL to use for authorization according to RFC 6749 section 3.1
        authorization_endpoint: String,
        /// The URL to use for obtaining an access token according to RFC 6749 section 3.2
        token_endpoint: String,
        /// The URL to use for retrieving user info
        user_info_endpoint: String,
        /// The scopes to request when authorizing
        scopes: String,
    },
}

impl Debug for ProviderConfiguration {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OpenIDConnect {
                client_id,
                authorization_endpoint,
                token_endpoint,
                user_info_endpoint,
                scopes,
                ..
            } => f
                .debug_struct("OpenIDConnect")
                .field("client_id", &client_id)
                .field("client_secret", &"<REDACTED>")
                .field("authorization_endpoint", &authorization_endpoint)
                .field("token_endpoint", &token_endpoint)
                .field("user_info_endpoint", &user_info_endpoint)
                .field("scopes", &scopes)
                .finish(),
        }
    }
}

impl Provider {
    /// Get all the providers
    #[instrument(name = "Provider::all", skip_all)]
    pub async fn all(db: &PgPool) -> sqlx::Result<Vec<Provider>> {
        query_as!(
            Provider,
            r#"
            SELECT 
                slug, enabled, name, icon, 
                config as "config: Json<ProviderConfiguration>", 
                created_at, updated_at
            FROM providers
            "#,
        )
        .fetch_all(db)
        .await
    }

    /// Get all the enabled providers
    #[instrument(name = "Provider::all_enabled", skip_all)]
    pub async fn all_enabled(db: &PgPool) -> sqlx::Result<Vec<Provider>> {
        query_as!(
            Provider,
            r#"
            SELECT 
                slug, enabled, name, icon, 
                config as "config: Json<ProviderConfiguration>", 
                created_at, updated_at
            FROM providers
            WHERE enabled = true
            "#,
        )
        .fetch_all(db)
        .await
    }

    /// Get a provider by it's slug
    #[instrument(name = "Provider::find", skip(db))]
    pub async fn find(slug: &str, db: &PgPool) -> sqlx::Result<Option<Provider>> {
        query_as!(
            Provider,
            r#"
            SELECT 
                slug, enabled, name, icon, 
                config as "config: Json<ProviderConfiguration>", 
                created_at, updated_at
            FROM providers
            WHERE slug = $1
            "#,
            slug,
        )
        .fetch_optional(db)
        .await
    }

    /// Get an enabled provider by it's slug
    #[instrument(name = "Provider::find_enabled", skip(db))]
    pub async fn find_enabled(slug: &str, db: &PgPool) -> sqlx::Result<Option<Provider>> {
        query_as!(
            Provider,
            r#"
            SELECT 
                slug, enabled, name, icon, 
                config as "config: Json<ProviderConfiguration>", 
                created_at, updated_at
            FROM providers
            WHERE slug = $1 AND enabled = true
            "#,
            slug,
        )
        .fetch_optional(db)
        .await
    }

    /// Create a new provider
    #[instrument(name = "Provider::create", skip(db))]
    pub async fn create(
        slug: &str,
        name: &str,
        icon: &str,
        config: ProviderConfiguration,
        db: &PgPool,
    ) -> sqlx::Result<Provider> {
        query_as!(
            Provider,
            r#"
            INSERT INTO providers (slug, name, icon, config) 
            VALUES ($1, $2, $3, $4) 
            RETURNING 
                slug, enabled, name, icon, 
                config as "config: Json<ProviderConfiguration>", 
                created_at, updated_at
        "#,
            slug,
            name,
            icon,
            Json(config) as _,
        )
        .fetch_one(db)
        .await
    }

    /// Update the display name and icon URL
    #[instrument(name = "Provider::set_display_details", skip(self, db), fields(%self.slug))]
    pub async fn set_display_details(
        &mut self,
        name: String,
        icon: String,
        db: &PgPool,
    ) -> sqlx::Result<()> {
        query!(
            "UPDATE providers SET name = $2, icon = $3 WHERE slug = $1",
            &self.slug,
            &name,
            &icon,
        )
        .execute(db)
        .await?;

        self.name = name;
        self.icon = icon;

        Ok(())
    }

    /// Update the enabled status
    #[instrument(name = "Provider::set_enabled", skip(self, db), fields(%self.slug))]
    pub async fn set_enabled(&mut self, enabled: bool, db: &PgPool) -> sqlx::Result<()> {
        query!(
            "UPDATE providers SET enabled = $2 WHERE slug = $1",
            &self.slug,
            enabled,
        )
        .execute(db)
        .await?;

        self.enabled = enabled;

        Ok(())
    }

    /// Set the provider configuration
    #[instrument(name = "Provider::set_config", skip(self, db), fields(%self.slug))]
    pub async fn set_config(
        &mut self,
        config: ProviderConfiguration,
        db: &PgPool,
    ) -> sqlx::Result<()> {
        let config = Json(config);

        query!(
            "UPDATE providers SET config = $2 WHERE slug = $1",
            &self.slug,
            &config as _,
        )
        .execute(db)
        .await?;

        self.config = config;

        Ok(())
    }

    /// Delete a provider by it's slug
    #[instrument(name = "Provider::delete", skip(db))]
    pub async fn delete(slug: &str, db: &PgPool) -> sqlx::Result<()> {
        query!("DELETE FROM providers WHERE slug = $1", slug)
            .execute(db)
            .await?;

        Ok(())
    }
}
