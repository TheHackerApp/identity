use super::{results, validators, UserError};
use async_graphql::{Context, ErrorExtensions, InputObject, Object, Result, ResultExt};
use database::{loaders::EventLoader, Event, Organization, PgPool};
use tracing::instrument;

results! {
    CreateEventResult {
        /// The created event
        event: Event,
    }
    UpdateEventResult {
        /// The event
        event: Event,
    }
    DeleteEventResult {
        /// The slug of the deleted event
        deleted_slug: String,
    }
}

#[derive(Default)]
pub(crate) struct EventMutation;

#[Object]
impl EventMutation {
    /// Create a new event
    #[instrument(name = "Mutation::create_event", skip(self, ctx))]
    async fn create_event(
        &self,
        ctx: &Context<'_>,
        input: CreateEventInput,
    ) -> Result<CreateEventResult> {
        let mut user_errors = Vec::new();

        if input.slug.is_empty() {
            user_errors.push(UserError::new(&["slug"], "cannot be empty"));
        }
        if input.slug.len() > 63 {
            user_errors.push(UserError::new(&["slug"], "must be less than 63 characters"));
        }
        if !validators::dns_segment(&input.slug) {
            user_errors.push(UserError::new(&["slug"], "must be a valid dns segment"));
        }
        if input.name.is_empty() {
            user_errors.push(UserError::new(&["name"], "cannot be empty"));
        }

        if !user_errors.is_empty() {
            return Ok(user_errors.into());
        }

        let db = ctx.data_unchecked::<PgPool>();

        if !Organization::exists(input.organization_id, db)
            .await
            .extend()?
        {
            return Ok(UserError::new(&["organization_id"], "organization does not exist").into());
        }

        match Event::create(&input.slug, &input.name, input.organization_id, db).await {
            Ok(organization) => Ok(organization.into()),
            Err(e) if e.is_unique_violation() => {
                Ok(UserError::new(&["slug"], "already in use").into())
            }
            Err(e) => Err(e.extend()),
        }
    }

    /// Update the details of an event
    #[instrument(name = "Mutation::update_event", skip(self, ctx))]
    async fn update_event(
        &self,
        ctx: &Context<'_>,
        input: UpdateEventInput,
    ) -> Result<UpdateEventResult> {
        if let Some(name) = &input.name {
            if name.is_empty() {
                return Ok(UserError::new(&["name"], "cannot be empty").into());
            }
        }

        let loader = ctx.data_unchecked::<EventLoader>();
        let Some(mut event) = loader.load_one(input.slug).await.extend()? else {
            return Ok(UserError::new(&["slug"], "event does not exist").into());
        };

        let db = ctx.data_unchecked::<PgPool>();
        event
            .update()
            .override_name(input.name)
            .save(db)
            .await
            .extend()?;

        Ok(event.into())
    }

    /// Delete an event
    #[instrument(name = "Mutation::delete_event", skip(self, ctx))]
    async fn delete_event(&self, ctx: &Context<'_>, slug: String) -> Result<DeleteEventResult> {
        let db = ctx.data::<PgPool>()?;
        Event::delete(&slug, db).await.extend()?;

        Ok(slug.into())
    }
}

/// Input fields for creating an event
#[derive(Debug, InputObject)]
struct CreateEventInput {
    /// A unique slug
    slug: String,
    /// The display name
    name: String,
    /// The organization putting on the event
    organization_id: i32,
}

/// Input fields for updating an event
#[derive(Debug, InputObject)]
struct UpdateEventInput {
    /// The slug of the event to update
    slug: String,
    /// The display name
    name: Option<String>,
}
