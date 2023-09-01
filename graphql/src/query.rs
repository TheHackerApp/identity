use async_graphql::{Context, Object, Result, ResultExt};
use database::{PgPool, Provider};
use tracing::instrument;

pub struct Query;

#[Object]
impl Query {
    /// Get all the authentication providers
    #[instrument(name = "Query::providers", skip_all)]
    async fn providers(&self, ctx: &Context<'_>) -> Result<Vec<Provider>> {
        let db = ctx.data::<PgPool>()?;
        let providers = Provider::all(&db).await.extend()?;

        Ok(providers)
    }

    /// Get an authentication provider by it's slug
    #[instrument(name = "Query::provider", skip(self, ctx))]
    async fn provider(&self, ctx: &Context<'_>, slug: String) -> Result<Option<Provider>> {
        let db = ctx.data::<PgPool>()?;
        let provider = Provider::find(&slug, &db).await.extend()?;

        Ok(provider)
    }
}
