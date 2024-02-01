use async_graphql::{Context, Error, Object, OneofObject, Result, ResultExt};
use context::{checks, guard, Scope, User as UserContext};
use database::{
    loaders::{
        EventLoader, OrganizationLoader, OrganizationsForUserLoader, ProviderLoader,
        UserByPrimaryEmailLoader, UserLoader,
    },
    Event, Organization, PgPool, Provider, User,
};
use tracing::instrument;

pub struct Query;

#[Object]
impl Query {
    /// Get all the authentication providers
    #[instrument(name = "Query::providers", skip_all)]
    async fn providers(&self, ctx: &Context<'_>) -> Result<Vec<Provider>> {
        let db = ctx.data_unchecked::<PgPool>();
        let providers = match checks::admin_only(ctx) {
            Ok(()) => Provider::all(db).await,
            Err(_) => Provider::all_enabled(db).await,
        }
        .extend()?;

        Ok(providers)
    }

    /// Get an authentication provider by its slug
    #[instrument(name = "Query::provider", skip(self, ctx))]
    #[graphql(guard = "guard(checks::admin_only)")]
    async fn provider(&self, ctx: &Context<'_>, slug: String) -> Result<Option<Provider>> {
        let loader = ctx.data_unchecked::<ProviderLoader>();
        let provider = loader.load_one(slug).await.extend()?;

        Ok(provider)
    }

    /// Get a user by their ID
    #[instrument(name = "Query::user", skip(self, ctx))]
    #[graphql(guard = "guard(checks::admin_only)")]
    async fn user(&self, ctx: &Context<'_>, by: UserBy) -> Result<Option<User>> {
        let user = match by {
            UserBy::Id(id) => {
                let loader = ctx.data_unchecked::<UserLoader>();
                loader.load_one(id).await
            }
            UserBy::PrimaryEmail(email) => {
                let loader = ctx.data_unchecked::<UserByPrimaryEmailLoader>();
                loader.load_one(email).await
            }
        }
        .extend()?;

        Ok(user)
    }

    /// Get all the registered organizations
    #[instrument(name = "Query::organizations", skip_all)]
    #[graphql(guard = "guard(checks::admin_only)")]
    async fn organizations(&self, ctx: &Context<'_>) -> Result<Vec<Organization>> {
        let db = ctx.data_unchecked::<PgPool>();
        let organizations = Organization::all(db).await.extend()?;

        Ok(organizations)
    }

    /// Get an organization by its ID
    #[instrument(name = "Query::organization", skip(self, ctx))]
    async fn organization(
        &self,
        ctx: &Context<'_>,
        id: Option<i32>,
    ) -> Result<Option<Organization>> {
        let scope = ctx.data_unchecked::<Scope>();
        let id = match (scope, id) {
            (Scope::Event(e), Some(id)) if e.organization_id == id => id,
            (Scope::Event(e), None) => e.organization_id,
            (_, Some(id)) => {
                checks::is_admin(ctx)?;
                id
            }
            (_, None) => {
                return Err(Error::new(
                    r#"argument "id" is required as the event could not be inferred"#,
                ));
            }
        };

        let loader = ctx.data_unchecked::<OrganizationLoader>();
        let organization = loader.load_one(id).await?;

        Ok(organization)
    }

    /// Get all the events being put on
    #[instrument(name = "Query::events", skip_all)]
    #[graphql(guard = "guard(checks::is_admin)")]
    async fn events(&self, ctx: &Context<'_>) -> Result<Vec<Event>> {
        let db = ctx.data_unchecked::<PgPool>();
        let events = Event::all(db).await?;

        Ok(events)
    }

    /// Get an event by its slug
    #[instrument(name = "Query::event", skip(self, ctx))]
    async fn event(&self, ctx: &Context<'_>, slug: Option<String>) -> Result<Option<Event>> {
        let scope = ctx.data_unchecked::<Scope>();
        let slug = match (scope, slug) {
            (Scope::Event(e), Some(slug)) if e.event == slug => slug,
            (Scope::Event(e), None) => e.event.to_owned(),
            (_, Some(slug)) => {
                checks::is_admin(ctx)?;
                slug
            }
            (_, None) => {
                return Err(Error::new(
                    r#"argument "slug" is required as the event could not be inferred"#,
                ));
            }
        };

        let loader = ctx.data_unchecked::<EventLoader>();
        let event = loader.load_one(slug).await?;

        Ok(event)
    }

    #[graphql(entity)]
    #[instrument(name = "Query::event_entity_by_slug", skip(self, ctx))]
    async fn event_entity_by_slug(
        &self,
        ctx: &Context<'_>,
        #[graphql(key)] slug: String,
    ) -> Result<Option<Event>> {
        let loader = ctx.data_unchecked::<EventLoader>();
        let event = loader.load_one(slug).await.extend()?;
        Ok(event)
    }

    #[graphql(entity)]
    #[instrument(name = "Query::organization_entity_by_id", skip(self, ctx))]
    async fn organization_entity_by_id(
        &self,
        ctx: &Context<'_>,
        #[graphql(key)] id: i32,
    ) -> Result<Option<Organization>> {
        let loader = ctx.data_unchecked::<OrganizationLoader>();
        let organization = loader.load_one(id).await.extend()?;
        Ok(organization)
    }

    #[graphql(entity)]
    #[instrument(name = "Query::user_entity_by_id", skip(self, ctx))]
    async fn user_entity_by_id(
        &self,
        ctx: &Context<'_>,
        #[graphql(key)] id: i32,
    ) -> Result<Option<User>> {
        let loader = ctx.data_unchecked::<UserLoader>();
        let user = loader.load_one(id).await.extend()?;
        Ok(user)
    }
}

/// How to look up a user
#[derive(Debug, OneofObject)]
enum UserBy {
    /// By ID
    Id(i32),
    /// By primary email
    PrimaryEmail(String),
}
