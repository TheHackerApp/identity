use crate::Result;
#[cfg(feature = "graphql")]
use crate::{loaders::EventLoader, Event};
#[cfg(feature = "graphql")]
use async_graphql::ResultExt;
use chrono::{DateTime, Utc};
#[cfg(feature = "graphql")]
use futures::TryStreamExt;
use sqlx::{query, query_as, PgPool, QueryBuilder};
#[cfg(feature = "graphql")]
use std::collections::HashMap;
use tracing::instrument;

/// A custom domain the event is accessible at
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
#[cfg_attr(feature = "graphql", graphql(complex))]
pub struct CustomDomain {
    /// The event the domain maps to
    #[cfg_attr(feature = "graphql", graphql(skip))]
    pub event: String,
    /// The domain name for the event
    pub name: String,
    // TODO: add verification fields
    /// When the custom domain was first created
    pub created_at: DateTime<Utc>,
    /// When the custom domain was last updated
    pub updated_at: DateTime<Utc>,
}

impl CustomDomain {
    /// Get all the custom domains
    #[instrument(name = "CustomDomain::all", skip_all)]
    pub async fn all(db: &PgPool) -> Result<Vec<CustomDomain>> {
        let domains = query_as!(CustomDomain, "SELECT * FROM custom_domains")
            .fetch_all(db)
            .await?;

        Ok(domains)
    }

    /// Load all the custom domains by their events' slugs, for use in dataloaders
    #[cfg(feature = "graphql")]
    pub(crate) async fn load(
        slugs: &[String],
        db: &PgPool,
    ) -> Result<HashMap<String, CustomDomain>> {
        let by_slug = query_as!(
            CustomDomain,
            "SELECT * FROM custom_domains WHERE event = ANY($1)",
            slugs
        )
        .fetch(db)
        .map_ok(|custom_domain| (custom_domain.event.clone(), custom_domain))
        .try_collect()
        .await?;

        Ok(by_slug)
    }

    /// Get the custom domain for an event
    #[instrument(name = "CustomDomain::find", skip(db))]
    pub async fn find(slug: &str, db: &PgPool) -> Result<Option<CustomDomain>> {
        let domain = query_as!(
            CustomDomain,
            "SELECT * FROM custom_domains WHERE event = $1",
            slug
        )
        .fetch_optional(db)
        .await?;

        Ok(domain)
    }

    /// Get a custom domain by it's name
    #[instrument(name = "CustomDomain::find_by_name", skip(db))]
    pub async fn find_by_name(name: &str, db: &PgPool) -> Result<Option<CustomDomain>> {
        let domain = query_as!(
            CustomDomain,
            "SELECT * FROM custom_domains WHERE name = $1",
            name
        )
        .fetch_optional(db)
        .await?;

        Ok(domain)
    }

    /// Create a new custom domain
    #[instrument(name = "CustomDomain::create", skip(db))]
    pub async fn create(name: &str, event: &str, db: &PgPool) -> Result<CustomDomain> {
        let domain = query_as!(
            CustomDomain,
            "INSERT INTO custom_domains (name, event) VALUES ($1, $2) RETURNING *",
            name,
            event
        )
        .fetch_one(db)
        .await?;

        Ok(domain)
    }

    /// Update the fields of a custom domain
    pub fn update(&mut self) -> CustomDomainUpdater<'_> {
        CustomDomainUpdater::new(self)
    }

    /// Delete the custom domain for an event
    #[instrument(name = "CustomDomain::delete", skip(db))]
    pub async fn delete(slug: &str, db: &PgPool) -> Result<()> {
        query!("DELETE FROM custom_domains WHERE event = $1", slug)
            .execute(db)
            .await?;

        Ok(())
    }
}

#[cfg(feature = "graphql")]
#[async_graphql::ComplexObject]
impl CustomDomain {
    /// The event that the custom domain is attached to
    #[instrument(name = "CustomDomain::event", skip_all)]
    async fn event(&self, ctx: &async_graphql::Context<'_>) -> async_graphql::Result<Event> {
        let loader = ctx.data_unchecked::<EventLoader>();
        let event = loader
            .load_one(self.event.clone())
            .await
            .extend()?
            .expect("custom domain must have associated event");

        Ok(event)
    }
}

/// Handles updating individual fields of the custom domain
pub struct CustomDomainUpdater<'c> {
    custom_domain: &'c mut CustomDomain,
    name: Option<String>,
}

impl<'c> CustomDomainUpdater<'c> {
    fn new(custom_domain: &'c mut CustomDomain) -> Self {
        Self {
            custom_domain,
            name: None,
        }
    }

    /// Set the domain name
    pub fn name(mut self, name: String) -> Self {
        self.name = Some(name);
        self
    }

    /// Override the domain name
    pub fn override_name(mut self, name: Option<String>) -> Self {
        self.name = name;
        self
    }

    /// Perform the update
    #[instrument(name = "CustomDomain::update", skip_all, fields(self.event = %self.custom_domain.event))]
    pub async fn save(self, db: &PgPool) -> Result<()> {
        if self.name.is_none() {
            // nothing changed
            return Ok(());
        }

        let mut builder = QueryBuilder::new("UPDATE custom_domains SET ");
        let mut separated = builder.separated(", ");

        if let Some(name) = &self.name {
            separated.push("name = ");
            separated.push_bind_unseparated(name);
        }

        builder.push(" WHERE event = ");
        builder.push_bind(&self.custom_domain.event);
        builder.build().execute(db).await?;

        if let Some(name) = self.name {
            self.custom_domain.name = name;
        }

        Ok(())
    }
}
