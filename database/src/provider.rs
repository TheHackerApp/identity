use crate::{Json, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{query, query_as, PgPool, QueryBuilder};
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
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum ProviderConfiguration {
    /// Google OpenID Connect provider
    Google {
        /// The client ID
        client_id: String,
        /// The client secret
        client_secret: String,
    },
    /// GitHub OAuth2 provider
    GitHub {
        /// The client ID
        client_id: String,
        /// The client secret
        client_secret: String,
    },
    /// Discord OAuth2 provider
    Discord {
        /// The client ID
        client_id: String,
        /// The client secret
        client_secret: String,
    },
}

impl ProviderConfiguration {
    /// Get the kind of provider
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Google { .. } => "google",
            Self::GitHub { .. } => "github",
            Self::Discord { .. } => "discord",
        }
    }
}

impl Debug for ProviderConfiguration {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Google { client_id, .. } => f
                .debug_struct("Google")
                .field("client_id", &client_id)
                .field("client_secret", &"<REDACTED>")
                .finish(),
            Self::GitHub { client_id, .. } => f
                .debug_struct("GitHub")
                .field("client_id", &client_id)
                .field("client_secret", &"<REDACTED>")
                .finish(),
            Self::Discord { client_id, .. } => f
                .debug_struct("Discord")
                .field("client_id", &client_id)
                .field("client_secret", &"<REDACTED>")
                .finish(),
        }
    }
}

impl Provider {
    /// Get all the providers
    #[instrument(name = "Provider::all", skip_all)]
    pub async fn all(db: &PgPool) -> Result<Vec<Provider>> {
        let providers = query_as!(
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
        .await?;
        Ok(providers)
    }

    /// Get all the enabled providers
    #[instrument(name = "Provider::all_enabled", skip_all)]
    pub async fn all_enabled(db: &PgPool) -> Result<Vec<Provider>> {
        let providers = query_as!(
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
        .await?;
        Ok(providers)
    }

    /// Get a provider by it's slug
    #[instrument(name = "Provider::find", skip(db))]
    pub async fn find(slug: &str, db: &PgPool) -> Result<Option<Provider>> {
        let provider = query_as!(
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
        .await?;
        Ok(provider)
    }

    /// Get an enabled provider by it's slug
    #[instrument(name = "Provider::find_enabled", skip(db))]
    pub async fn find_enabled(slug: &str, db: &PgPool) -> Result<Option<Provider>> {
        let provider = query_as!(
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
        .await?;
        Ok(provider)
    }

    /// Create a new provider
    #[instrument(name = "Provider::create", skip(db))]
    pub async fn create(
        slug: &str,
        name: &str,
        icon: &str,
        config: ProviderConfiguration,
        db: &PgPool,
    ) -> Result<Provider> {
        let provider = query_as!(
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
        .await?;
        Ok(provider)
    }

    /// Update the fields of a provider
    pub fn update(&mut self) -> ProviderUpdater<'_> {
        ProviderUpdater::new(self)
    }

    /// Delete a provider by it's slug
    #[instrument(name = "Provider::delete", skip(db))]
    pub async fn delete(slug: &str, db: &PgPool) -> Result<()> {
        query!("DELETE FROM providers WHERE slug = $1", slug)
            .execute(db)
            .await?;

        Ok(())
    }
}

/// Handles updating individual fields of the provider
pub struct ProviderUpdater<'p> {
    provider: &'p mut Provider,
    enabled: Option<bool>,
    name: Option<String>,
    icon: Option<String>,
    config: Option<Json<ProviderConfiguration>>,
}

impl<'p> ProviderUpdater<'p> {
    fn new(provider: &'p mut Provider) -> ProviderUpdater<'p> {
        Self {
            provider,
            enabled: None,
            name: None,
            icon: None,
            config: None,
        }
    }

    /// Update the enabled status
    pub fn enabled(mut self, enabled: bool) -> ProviderUpdater<'p> {
        self.enabled = Some(enabled);
        self
    }

    /// Directly set the enabled status
    pub fn override_enabled(mut self, enabled: Option<bool>) -> ProviderUpdater<'p> {
        self.enabled = enabled;
        self
    }

    /// Update the display name
    pub fn name(mut self, name: String) -> ProviderUpdater<'p> {
        self.name = Some(name);
        self
    }

    /// Directly set the display name
    pub fn override_name(mut self, name: Option<String>) -> ProviderUpdater<'p> {
        self.name = name;
        self
    }

    /// Update the icon
    pub fn icon(mut self, icon: String) -> ProviderUpdater<'p> {
        self.icon = Some(icon);
        self
    }

    /// Directly set the icon
    pub fn override_icon(mut self, icon: Option<String>) -> ProviderUpdater<'p> {
        self.icon = icon;
        self
    }

    /// Update the provider-specific configuration
    pub fn config(mut self, config: ProviderConfiguration) -> ProviderUpdater<'p> {
        self.config = Some(Json(config));
        self
    }

    /// Directly set the provider-specific configuration
    pub fn override_config(
        mut self,
        config: Option<Json<ProviderConfiguration>>,
    ) -> ProviderUpdater<'p> {
        self.config = config;
        self
    }

    /// Perform the update
    #[instrument(name = "Provider::update", skip_all, fields(self.slug = %self.provider.slug))]
    pub async fn save(self, db: &PgPool) -> Result<()> {
        if self.enabled.is_none()
            && self.name.is_none()
            && self.icon.is_none()
            && self.config.is_none()
        {
            // nothing was changed
            return Ok(());
        }

        let mut builder = QueryBuilder::new("UPDATE providers SET ");
        let mut separated = builder.separated(", ");

        if let Some(enabled) = &self.enabled {
            separated.push("enabled = ");
            separated.push_bind_unseparated(enabled);
        }

        if let Some(name) = &self.name {
            separated.push("name = ");
            separated.push_bind_unseparated(name);
        }

        if let Some(icon) = &self.icon {
            separated.push("icon = ");
            separated.push_bind_unseparated(icon);
        }

        if let Some(config) = &self.config {
            separated.push("config = ");
            separated.push_bind_unseparated(config);
        }

        builder.push(" WHERE slug = ");
        builder.push_bind(&self.provider.slug);
        builder.build().execute(db).await?;

        if let Some(enabled) = self.enabled {
            self.provider.enabled = enabled;
        }

        if let Some(name) = self.name {
            self.provider.name = name;
        }

        if let Some(icon) = self.icon {
            self.provider.icon = icon;
        }

        if let Some(config) = self.config {
            self.provider.config = config;
        }

        Ok(())
    }
}
