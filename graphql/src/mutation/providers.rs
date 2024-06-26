use super::{results, validators, UserError};
use async_graphql::{Context, ErrorExtensions, InputObject, Object, Result, ResultExt};
use database::{loaders::ProviderLoader, Json, PgPool, Provider, ProviderConfiguration};
use tracing::instrument;

results! {
    CreateProviderResult {
        /// The created authentication provider
        provider: Provider,
    }
    UpdateProviderResult {
        /// The authentication provider
        provider: Provider,
    }
    DeleteProviderResult {
        /// The slug of the deleted authentication provider
        deleted_slug: String,
    }
}

#[derive(Default)]
pub(crate) struct ProviderMutation;

#[Object]
impl ProviderMutation {
    /// Add a new authentication provider. The provider will be disabled by default.
    #[instrument(name = "Mutation::create_provider", skip(self, ctx))]
    async fn create_provider(
        &self,
        ctx: &Context<'_>,
        input: CreateProviderInput,
    ) -> Result<CreateProviderResult> {
        let mut user_errors = Vec::new();

        if input.slug.is_empty() {
            user_errors.push(UserError::new(&["slug"], "cannot be empty"));
        }
        if !validators::identifier(&input.slug) {
            user_errors.push(UserError::new(&["slug"], "must be a valid identifier"))
        }
        if input.name.is_empty() {
            user_errors.push(UserError::new(&["name"], "cannot be empty"));
        }

        if !user_errors.is_empty() {
            return Ok(user_errors.into());
        }

        let db = ctx.data_unchecked::<PgPool>();
        match Provider::create(&input.slug, &input.name, input.config.0, db).await {
            Ok(provider) => Ok(provider.into()),
            Err(e) if e.is_unique_violation() => {
                Ok(UserError::new(&["slug"], "already in use").into())
            }
            Err(e) => Err(e.extend()),
        }
    }

    /// Update the details of an authentication provider
    #[instrument(name = "Mutation::update_provider", skip(self, ctx))]
    async fn update_provider(
        &self,
        ctx: &Context<'_>,
        input: UpdateProviderInput,
    ) -> Result<UpdateProviderResult> {
        let mut user_errors = Vec::new();

        if let Some(name) = &input.name {
            if name.is_empty() {
                user_errors.push(UserError::new(&["name"], "cannot be empty"));
            }
        }

        if !user_errors.is_empty() {
            return Ok(user_errors.into());
        }

        let loader = ctx.data_unchecked::<ProviderLoader>();
        let Some(mut provider) = loader.load_one(input.slug).await.extend()? else {
            return Ok(UserError::new(&["slug"], "provider does not exist").into());
        };

        let db = ctx.data_unchecked::<PgPool>();
        provider
            .update()
            .override_enabled(input.enabled)
            .override_name(input.name)
            .override_config(input.config)
            .save(db)
            .await
            .extend()?;

        Ok(provider.into())
    }

    /// Delete an authentication provider
    #[instrument(name = "Mutation::delete_provider", skip(self, ctx))]
    async fn delete_provider(
        &self,
        ctx: &Context<'_>,
        slug: String,
    ) -> Result<DeleteProviderResult> {
        let db = ctx.data_unchecked::<PgPool>();
        Provider::delete(&slug, db).await.extend()?;

        Ok(slug.into())
    }
}

/// Input fields for creating a provider
#[derive(Debug, InputObject)]
struct CreateProviderInput {
    /// A unique slug
    slug: String,
    /// The public-facing display name
    name: String,
    /// The provider-specific configuration
    // TODO: create specialized input-type for configuration
    config: Json<ProviderConfiguration>,
}

/// Input fields for updating a provider
#[derive(Debug, InputObject)]
struct UpdateProviderInput {
    /// The slug of the provider to update
    slug: String,
    /// Whether the provider can be used
    enabled: Option<bool>,
    /// The public-facing display name
    name: Option<String>,
    /// The provider-specific configuration
    config: Option<Json<ProviderConfiguration>>,
}
