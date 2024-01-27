use crate::Result;
#[cfg(feature = "graphql")]
use crate::{
    loaders::{EventsForOrganizationLoader, UserLoader},
    Event, User,
};
#[cfg(feature = "graphql")]
use async_graphql::{Context, ResultExt};
use chrono::{DateTime, Utc};
#[cfg(feature = "graphql")]
use futures::TryStreamExt;
use sqlx::{query, query_as, PgPool, QueryBuilder};
#[cfg(feature = "graphql")]
use std::collections::HashMap;
use tracing::instrument;

/// An organization that puts on events
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
#[cfg_attr(feature = "graphql", graphql(complex))]
pub struct Organization {
    /// A unique ID
    pub id: i32,
    /// The name of the organization
    pub name: String,
    /// URL for the organization's logo
    pub logo: Option<String>,
    /// URL for the organization's website
    pub website: Option<String>,
    /// The user who owns the organization
    #[cfg_attr(feature = "graphql", graphql(skip))]
    pub owner_id: i32,
    /// When the organization was first created
    pub created_at: DateTime<Utc>,
    /// When the organization was last updated
    pub updated_at: DateTime<Utc>,
}

impl Organization {
    /// Get all the registered organizations
    #[instrument(name = "Organization::all", skip_all)]
    pub async fn all(db: &PgPool) -> Result<Vec<Organization>> {
        let organizations = query_as!(Organization, "SELECT * FROM organizations")
            .fetch_all(db)
            .await?;

        Ok(organizations)
    }

    /// Load all the organizations by the IDs, for use in dataloaders
    #[cfg(feature = "graphql")]
    pub(crate) async fn load(ids: &[i32], db: &PgPool) -> Result<HashMap<i32, Organization>> {
        let by_id = query_as!(
            Organization,
            "SELECT * FROM organizations WHERE id = ANY($1)",
            ids
        )
        .fetch(db)
        .map_ok(|organization| (organization.id, organization))
        .try_collect()
        .await?;
        Ok(by_id)
    }

    /// Check if an organization exists
    #[instrument(name = "Organization::exists", skip(db))]
    pub async fn exists(id: i32, db: &PgPool) -> Result<bool> {
        let result = query!(
            "SELECT exists(SELECT 1 FROM organizations WHERE id = $1)",
            id
        )
        .fetch_one(db)
        .await?;

        Ok(result.exists.unwrap_or_default())
    }

    /// Get an organization by it's ID
    #[instrument(name = "Organization::find", skip(db))]
    pub async fn find(id: i32, db: &PgPool) -> Result<Option<Organization>> {
        let organization = query_as!(
            Organization,
            "SELECT * FROM organizations WHERE id = $1",
            id
        )
        .fetch_optional(db)
        .await?;

        Ok(organization)
    }

    /// Create a new organization
    #[instrument(name = "Organization::create", skip(db))]
    pub async fn create(name: &str, owner_id: i32, db: &PgPool) -> Result<Organization> {
        let organization = query_as!(
            Organization,
            "INSERT INTO organizations (name, owner_id) VALUES ($1, $2) RETURNING *",
            name,
            owner_id
        )
        .fetch_one(db)
        .await?;

        Ok(organization)
    }

    /// Update the organization's fields
    pub fn update(&mut self) -> OrganizationUpdater<'_> {
        OrganizationUpdater::new(self)
    }

    /// Delete an organization
    #[instrument(name = "Organization::delete", skip(db))]
    pub async fn delete(id: i32, db: &PgPool) -> Result<()> {
        query!("DELETE FROM organizations WHERE id = $1", id)
            .execute(db)
            .await?;

        Ok(())
    }
}

// TODO: restrict to organization members/admins
#[cfg(feature = "graphql")]
#[async_graphql::ComplexObject]
impl Organization {
    /// All the events owned by the organization
    #[instrument(name = "Organization::events", skip_all, fields(%self.id))]
    async fn events(&self, ctx: &Context<'_>) -> async_graphql::Result<Vec<Event>> {
        let loader = ctx.data_unchecked::<EventsForOrganizationLoader>();
        let events = loader.load_one(self.id).await.extend()?.unwrap_or_default();

        Ok(events)
    }

    /// The owner of the organization
    #[instrument(name = "Organization::owner", skip_all, fields(%self.id))]
    async fn owner(&self, ctx: &Context<'_>) -> async_graphql::Result<User> {
        let loader = ctx.data_unchecked::<UserLoader>();
        let user = loader
            .load_one(self.owner_id)
            .await
            .extend()?
            .expect("organization must have an owner");

        Ok(user)
    }
}

/// Handles updating individual fields of the organization
pub struct OrganizationUpdater<'o> {
    organization: &'o mut Organization,
    name: Option<String>,
    logo: Option<Option<String>>,
    website: Option<Option<String>>,
    owner_id: Option<i32>,
}

impl<'o> OrganizationUpdater<'o> {
    fn new(organization: &'o mut Organization) -> OrganizationUpdater<'o> {
        Self {
            organization,
            name: None,
            logo: None,
            website: None,
            owner_id: None,
        }
    }

    /// Update the name
    pub fn name(mut self, name: String) -> OrganizationUpdater<'o> {
        self.name = Some(name);
        self
    }

    /// Directly set the name
    pub fn override_name(mut self, name: Option<String>) -> OrganizationUpdater<'o> {
        self.name = name;
        self
    }

    /// Set the logo URL
    pub fn logo(mut self, logo: Option<String>) -> OrganizationUpdater<'o> {
        self.logo = Some(logo);
        self
    }

    /// Override the logo URL
    pub fn override_logo(mut self, logo: Option<Option<String>>) -> OrganizationUpdater<'o> {
        self.logo = logo;
        self
    }

    /// Set the website URL
    pub fn website(mut self, website: Option<String>) -> OrganizationUpdater<'o> {
        self.website = Some(website);
        self
    }

    /// Override the website URL
    pub fn override_website(mut self, website: Option<Option<String>>) -> OrganizationUpdater<'o> {
        self.website = website;
        self
    }

    /// Set the owner ID
    pub fn owner(mut self, id: i32) -> OrganizationUpdater<'o> {
        self.owner_id = Some(id);
        self
    }

    /// Override the owner ID
    pub fn override_owner(mut self, id: Option<i32>) -> OrganizationUpdater<'o> {
        self.owner_id = id;
        self
    }

    /// Perform the update
    #[instrument(name = "Organization::update", skip_all, fields(self.id = self.organization.id))]
    pub async fn save(self, db: &PgPool) -> Result<()> {
        if self.name.is_none()
            && self.logo.is_none()
            && self.website.is_none()
            && self.owner_id.is_none()
        {
            // nothing was changed
            return Ok(());
        }

        let mut builder = QueryBuilder::new("UPDATE organizations SET ");
        let mut separated = builder.separated(", ");

        if let Some(name) = &self.name {
            separated.push("name = ");
            separated.push_bind_unseparated(name);
        }

        if let Some(logo) = &self.logo {
            separated.push("logo = ");
            separated.push_bind_unseparated(logo);
        }

        if let Some(website) = &self.website {
            separated.push("website = ");
            separated.push_bind_unseparated(website);
        }

        if let Some(owner_id) = self.owner_id {
            separated.push("owner_id = ");
            separated.push_bind_unseparated(owner_id);
        }

        builder.push(" WHERE id = ");
        builder.push_bind(self.organization.id);
        builder.build().execute(db).await?;

        if let Some(name) = self.name {
            self.organization.name = name;
        }

        if let Some(logo) = self.logo {
            self.organization.logo = logo;
        }

        if let Some(website) = self.website {
            self.organization.website = website;
        }

        if let Some(owner_id) = self.owner_id {
            self.organization.owner_id = owner_id;
        }

        Ok(())
    }
}
