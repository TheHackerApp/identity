use async_graphql::{Context, Object, OneofObject, Result, ResultExt};
use database::{PgPool, Provider, User};
use tracing::instrument;

pub struct Query;

#[Object]
impl Query {
    /// Get all the authentication providers
    #[instrument(name = "Query::providers", skip_all)]
    async fn providers(&self, ctx: &Context<'_>) -> Result<Vec<Provider>> {
        let db = ctx.data::<PgPool>()?;
        let providers = Provider::all(db).await.extend()?;

        Ok(providers)
    }

    /// Get an authentication provider by it's slug
    #[instrument(name = "Query::provider", skip(self, ctx))]
    async fn provider(&self, ctx: &Context<'_>, slug: String) -> Result<Option<Provider>> {
        let db = ctx.data::<PgPool>()?;
        let provider = Provider::find(&slug, db).await.extend()?;

        Ok(provider)
    }

    /// Get a user by their ID
    #[instrument(name = "Query::user", skip(self, ctx))]
    async fn user(&self, ctx: &Context<'_>, by: UserBy) -> Result<Option<User>> {
        let db = ctx.data::<PgPool>()?;
        let user = match by {
            UserBy::Id(id) => User::find(id, db).await,
            UserBy::PrimaryEmail(email) => User::find_by_primary_email(&email, db).await,
        }
        .extend()?;

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
