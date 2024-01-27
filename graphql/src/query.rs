use async_graphql::{Context, Object, OneofObject, Result, ResultExt};
use database::{
    loaders::{
        EventLoader, OrganizationLoader, ProviderLoader, UserByPrimaryEmailLoader, UserLoader,
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
        let providers = Provider::all(db).await.extend()?;

        Ok(providers)
    }

    /// Get an authentication provider by it's slug
    #[instrument(name = "Query::provider", skip(self, ctx))]
    async fn provider(&self, ctx: &Context<'_>, slug: String) -> Result<Option<Provider>> {
        let loader = ctx.data_unchecked::<ProviderLoader>();
        let provider = loader.load_one(slug).await.extend()?;

        Ok(provider)
    }

    /// Get a user by their ID
    #[instrument(name = "Query::user", skip(self, ctx))]
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

/// How to lookup a user
#[derive(Debug, OneofObject)]
enum UserBy {
    /// By ID
    Id(i32),
    /// By primary email
    PrimaryEmail(String),
}
