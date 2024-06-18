use crate::Result;
#[cfg(feature = "graphql")]
use crate::{
    loaders::{CustomDomainLoader, OrganizationLoader},
    CustomDomain, Organization,
};
#[cfg(feature = "graphql")]
use async_graphql::ResultExt;
use chrono::{DateTime, Utc};
#[cfg(feature = "graphql")]
use context::{
    checks::{guard_where, has_at_least_role},
    UserRole,
};
#[cfg(feature = "graphql")]
use futures::TryStreamExt;
use sqlx::{query, query_as, Executor, QueryBuilder};
#[cfg(feature = "graphql")]
use state::Domains;
#[cfg(feature = "graphql")]
use std::collections::HashMap;
use tracing::instrument;

/// An event that is put on
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
#[cfg_attr(feature = "graphql", graphql(complex))]
pub struct Event {
    /// The unique slug
    pub slug: String,
    /// Display name of the event
    pub name: String,
    /// The organization that owns the event
    #[cfg_attr(feature = "graphql", graphql(skip))]
    pub organization_id: i32,
    /// When write-access expires
    #[cfg_attr(
        feature = "graphql",
        graphql(guard = "guard_where(has_at_least_role, UserRole::Organizer)")
    )]
    pub expires_on: DateTime<Utc>,
    /// When the event was first created
    pub created_at: DateTime<Utc>,
    /// When the event was last updated
    pub updated_at: DateTime<Utc>,
}

impl Event {
    /// Get all the registered events
    #[instrument(name = "Event::all", skip_all)]
    pub async fn all<'c, 'e, E>(db: E) -> Result<Vec<Event>>
    where
        'c: 'e,
        E: 'e + Executor<'c, Database = sqlx::Postgres>,
    {
        let events = query_as!(Event, "SELECT * FROM events")
            .fetch_all(db)
            .await?;

        Ok(events)
    }

    /// Load all the events by their slugs, for use in dataloaders
    #[cfg(feature = "graphql")]
    pub(crate) async fn load<'c, 'e, E>(slugs: &[String], db: E) -> Result<HashMap<String, Event>>
    where
        'c: 'e,
        E: 'e + Executor<'c, Database = sqlx::Postgres>,
    {
        let by_slug = query_as!(Event, "SELECT * FROM events WHERE slug = ANY($1)", slugs)
            .fetch(db)
            .map_ok(|event| (event.slug.clone(), event))
            .try_collect()
            .await?;
        Ok(by_slug)
    }

    /// Load all the events for the selected organizations by their IDs, for use in dataloaders
    #[cfg(feature = "graphql")]
    pub(crate) async fn load_for_organizations<'c, 'e, E>(
        organization_ids: &[i32],
        db: E,
    ) -> Result<HashMap<i32, Vec<Event>>>
    where
        'c: 'e,
        E: 'e + Executor<'c, Database = sqlx::Postgres>,
    {
        let by_organization = query_as!(
            Event,
            "SELECT * FROM events WHERE organization_id = ANY($1)",
            organization_ids
        )
        .fetch(db)
        .try_fold(HashMap::new(), |mut map, event| async {
            let entry: &mut Vec<Event> = map.entry(event.organization_id).or_default();
            entry.push(event);
            Ok(map)
        })
        .await?;
        Ok(by_organization)
    }

    /// Get all the events for an organization
    #[instrument(name = "Event::for_organization", skip(db))]
    pub async fn for_organization<'c, 'e, E>(organization_id: i32, db: E) -> Result<Vec<Event>>
    where
        'c: 'e,
        E: 'e + Executor<'c, Database = sqlx::Postgres>,
    {
        let events = query_as!(
            Event,
            "SELECT * FROM events WHERE organization_id = $1",
            organization_id
        )
        .fetch_all(db)
        .await?;

        Ok(events)
    }

    /// Check if an event exists
    #[instrument(name = "Event::exists", skip(db))]
    pub async fn exists<'c, 'e, E>(slug: &str, db: E) -> Result<bool>
    where
        'c: 'e,
        E: 'e + Executor<'c, Database = sqlx::Postgres>,
    {
        let result = query!("SELECT exists(SELECT 1 FROM events WHERE slug = $1)", slug)
            .fetch_one(db)
            .await?;

        Ok(result.exists.unwrap_or_default())
    }

    /// Get an event by it's slug
    #[instrument(name = "Event::find", skip(db))]
    pub async fn find<'c, 'e, E>(slug: &str, db: E) -> Result<Option<Event>>
    where
        'c: 'e,
        E: 'e + Executor<'c, Database = sqlx::Postgres>,
    {
        let event = query_as!(Event, "SELECT * FROM events WHERE slug = $1", slug)
            .fetch_optional(db)
            .await?;

        Ok(event)
    }

    /// Get an event by it's custom domain
    #[instrument(name = "Event::find_by_custom_domain", skip(db))]
    pub async fn find_by_custom_domain<'c, 'e, E>(name: &str, db: E) -> Result<Option<Event>>
    where
        'c: 'e,
        E: 'e + Executor<'c, Database = sqlx::Postgres>,
    {
        // TODO: ensure custom domain is valid

        let event = query_as!(
            Event,
            r#"
            SELECT events.* FROM events 
            INNER JOIN custom_domains ON events.slug = custom_domains.event 
            WHERE custom_domains.name = $1
            "#,
            name
        )
        .fetch_optional(db)
        .await?;

        Ok(event)
    }

    /// Create a new event
    #[instrument(name = "Event::create", skip(db))]
    pub async fn create<'c, 'e, E>(
        slug: &str,
        name: &str,
        organization_id: i32,
        db: E,
    ) -> Result<Event>
    where
        'c: 'e,
        E: 'e + Executor<'c, Database = sqlx::Postgres>,
    {
        let event = query_as!(
            Event,
            "INSERT INTO events (slug, name, organization_id) VALUES ($1, $2, $3) RETURNING *",
            slug,
            name,
            organization_id
        )
        .fetch_one(db)
        .await?;

        Ok(event)
    }

    /// Check if the event is active
    pub fn is_active(&self) -> bool {
        self.expires_on >= Utc::now()
    }

    /// Update the fields of an event
    pub fn update(&mut self) -> EventUpdater<'_> {
        EventUpdater::new(self)
    }

    /// Delete an event
    #[instrument(name = "Event::delete", skip(db))]
    pub async fn delete<'c, 'e, E>(slug: &str, db: E) -> Result<()>
    where
        'c: 'e,
        E: 'e + Executor<'c, Database = sqlx::Postgres>,
    {
        query!("DELETE FROM events WHERE slug = $1", slug)
            .execute(db)
            .await?;

        Ok(())
    }
}

#[cfg(feature = "graphql")]
#[async_graphql::ComplexObject]
impl Event {
    /// Whether the event is active
    async fn active(&self) -> bool {
        self.is_active()
    }

    /// The domain where the event is accessible
    #[instrument(name = "Event::domain", skip_all, fields(%self.slug))]
    async fn domain(&self, ctx: &async_graphql::Context<'_>) -> async_graphql::Result<String> {
        let loader = ctx.data_unchecked::<CustomDomainLoader>();
        let custom_domain = loader.load_one(self.slug.to_owned()).await.extend()?;

        Ok(match custom_domain {
            Some(custom) => custom.name,
            None => {
                let domains = ctx.data_unchecked::<Domains>();
                domains.for_event(&self.slug)
            }
        })
    }

    /// The custom domain for the event
    #[graphql(guard = "guard_where(has_at_least_role, UserRole::Organizer)")]
    #[instrument(name = "Event::custom_domain", skip_all, fields(%self.slug))]
    async fn custom_domain(
        &self,
        ctx: &async_graphql::Context<'_>,
    ) -> async_graphql::Result<Option<CustomDomain>> {
        let loader = ctx.data_unchecked::<CustomDomainLoader>();
        let custom_domain = loader.load_one(self.slug.to_owned()).await.extend()?;

        Ok(custom_domain)
    }

    /// The organization that owns the event
    #[instrument(name = "Event::organization", skip_all, fields(%self.slug))]
    async fn organization(
        &self,
        ctx: &async_graphql::Context<'_>,
    ) -> async_graphql::Result<Organization> {
        let loader = ctx.data_unchecked::<OrganizationLoader>();
        let organization = loader
            .load_one(self.organization_id)
            .await
            .extend()?
            .expect("event must have an associated organization");

        Ok(organization)
    }
}

/// Handles updating individual fields of the event
pub struct EventUpdater<'e> {
    event: &'e mut Event,
    name: Option<String>,
    organization_id: Option<i32>,
    expires_on: Option<DateTime<Utc>>,
}

impl<'e> EventUpdater<'e> {
    fn new(event: &'e mut Event) -> Self {
        Self {
            event,
            name: None,
            organization_id: None,
            expires_on: None,
        }
    }

    /// Set the display name
    pub fn name(mut self, name: String) -> Self {
        self.name = Some(name);
        self
    }

    /// Override the display name
    pub fn override_name(mut self, name: Option<String>) -> Self {
        self.name = name;
        self
    }

    /// Set the organization owner
    pub fn organization(mut self, id: i32) -> Self {
        self.organization_id = Some(id);
        self
    }

    /// Override the organization owner
    pub fn override_organization(mut self, id: Option<i32>) -> Self {
        self.organization_id = id;
        self
    }

    /// Set when the write-access expires
    pub fn expires_on(mut self, at: DateTime<Utc>) -> Self {
        self.expires_on = Some(at);
        self
    }

    /// Override when the write-access expires
    pub fn override_expires_on(mut self, at: Option<DateTime<Utc>>) -> Self {
        self.expires_on = at;
        self
    }

    /// Perform the update
    #[instrument(name = "Event::update", skip_all, fields(self.id = %self.event.slug))]
    pub async fn save<'c, 'ex, E>(self, db: E) -> Result<()>
    where
        'c: 'ex,
        E: 'ex + Executor<'c, Database = sqlx::Postgres>,
    {
        if self.name.is_none() && self.organization_id.is_none() && self.expires_on.is_none() {
            // nothing changed
            return Ok(());
        }

        let mut builder = QueryBuilder::new("UPDATE events SET ");
        let mut separated = builder.separated(", ");

        if let Some(name) = &self.name {
            separated.push("name = ");
            separated.push_bind_unseparated(name);
        }

        if let Some(organization_id) = self.organization_id {
            separated.push("organization_id = ");
            separated.push_bind_unseparated(organization_id);
        }

        if let Some(expires_on) = self.expires_on {
            separated.push("expires_on = ");
            separated.push_bind_unseparated(expires_on);
        }

        builder.push(" WHERE slug = ");
        builder.push_bind(&self.event.slug);
        builder.build().execute(db).await?;

        if let Some(name) = self.name {
            self.event.name = name;
        }

        if let Some(organization_id) = self.organization_id {
            self.event.organization_id = organization_id;
        }

        if let Some(expires_on) = self.expires_on {
            self.event.expires_on = expires_on;
        }

        Ok(())
    }
}
