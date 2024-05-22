use crate::{
    entities,
    errors::{Forbidden, Unauthorized},
};
use async_graphql::{Context, Error, Object, OneofObject, Result, ResultExt};
use context::{checks, guard, Scope, User as UserContext};
use database::{
    loaders::{
        EventLoader, OrganizationLoader, ProviderLoader, UserByPrimaryEmailLoader, UserLoader,
    },
    Event, Organization, Organizer, Participant, PgPool, Provider, User,
};
use tracing::instrument;

pub struct Query;

#[Object]
impl Query {
    /// Get information about the current user
    #[instrument(name = "Query::me", skip_all)]
    async fn me(&self, ctx: &Context<'_>) -> Result<User> {
        match ctx.data_unchecked::<UserContext>() {
            UserContext::Authenticated(user) => {
                let loader = ctx.data_unchecked::<UserLoader>();
                loader.load_one(user.id).await.extend().transpose().unwrap()
            }
            UserContext::OAuth | UserContext::RegistrationNeeded(_) => Err(Forbidden.into()),
            UserContext::Unauthenticated => Err(Unauthorized.into()),
        }
    }

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
            (Scope::Admin, Some(id)) => {
                checks::is_admin(ctx)?;
                id
            }
            (Scope::User, Some(id)) => {
                let db = ctx.data_unchecked::<PgPool>();
                let user = checks::is_authenticated(ctx)?;
                if User::is_organizer(user.id, id, db).await?.is_some() {
                    id
                } else {
                    return Err(Forbidden.into());
                }
            }
            (Scope::Event(e), Some(id)) if e.organization_id == id => id,
            (Scope::Event(e), None) => e.organization_id,
            (Scope::Event(_), Some(_)) => return Err(Forbidden.into()),
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
            (Scope::Admin, Some(slug)) => {
                checks::is_admin(ctx)?;
                slug
            }
            (Scope::User, Some(slug)) => {
                let db = ctx.data_unchecked::<PgPool>();
                let user = checks::is_authenticated(ctx)?;
                if User::is_organizer_for_event(user.id, &slug, db).await?
                    || User::is_participant(user.id, &slug, db).await?
                {
                    slug
                } else {
                    return Err(Forbidden.into());
                }
            }
            (Scope::Event(e), Some(slug)) if e.event == slug => slug,
            (Scope::Event(e), None) => e.event.to_owned(),
            (Scope::Event(_), Some(_)) => return Err(Forbidden.into()),
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
    #[instrument(name = "Query::entity::event", skip(self, ctx))]
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
    #[instrument(name = "Query::entity::organization", skip(self, ctx))]
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
    #[instrument(name = "Query::entity::user", skip(self, ctx))]
    async fn user_entity_by_id(
        &self,
        ctx: &Context<'_>,
        #[graphql(key)] id: i32,
    ) -> Result<Option<User>> {
        let loader = ctx.data_unchecked::<UserLoader>();
        let user = loader.load_one(id).await.extend()?;
        Ok(user)
    }

    #[graphql(entity)]
    #[instrument(name = "Query::entity::participant", skip(self, ctx))]
    async fn participant_entity_by_id(
        &self,
        ctx: &Context<'_>,
        #[graphql(key)] event: entities::Event,
        #[graphql(key)] user: entities::User,
    ) -> Result<Option<Participant>> {
        let db = ctx.data_unchecked::<PgPool>();
        let participant = Participant::find(user.id, &event.slug, db).await.extend()?;
        Ok(participant)
    }

    #[graphql(entity)]
    #[instrument(name = "Query::entity::organizer", skip(self, ctx))]
    async fn organizer_entity_by_id(
        &self,
        ctx: &Context<'_>,
        #[graphql(key)] organization: entities::Organization,
        #[graphql(key)] user: entities::User,
    ) -> Result<Option<Organizer>> {
        let db = ctx.data_unchecked::<PgPool>();
        let organizer = Organizer::find(user.id, organization.id, db)
            .await
            .extend()?;
        Ok(organizer)
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
