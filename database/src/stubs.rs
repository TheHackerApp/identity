use crate::{
    loaders::{UsersForEventLoader, UsersForOrganizationLoader},
    Organizer, Participant,
};
use async_graphql::{ComplexObject, Context, Result, ResultExt, SimpleObject};
use tracing::instrument;

// TODO: remove stubs entirely

/// A user of the service
#[derive(Clone, Debug, Eq, PartialEq, SimpleObject)]
#[graphql(unresolvable)]
pub struct User {
    pub id: i32,
}

/// An organization that puts on events
#[derive(Debug, SimpleObject)]
#[graphql(complex)]
pub struct Organization {
    pub id: i32,
}

#[ComplexObject]
impl Organization {
    /// The users that are part of the organization
    #[instrument(name = "Organization::members", skip(self, ctx), fields(organization.id = self.id))]
    async fn members(&self, ctx: &Context<'_>) -> Result<Vec<Organizer>> {
        let loader = ctx.data_unchecked::<UsersForOrganizationLoader>();
        let members = loader.load_one(self.id).await.extend()?.unwrap_or_default();

        Ok(members)
    }
}

/// An event that is put on
#[derive(Debug, SimpleObject)]
#[graphql(complex)]
pub struct Event {
    pub slug: String,
}

#[ComplexObject]
impl Event {
    /// The users that are part of the event
    #[instrument(name = "Event::participants", skip(self, ctx), fields(event.slug = %self.slug))]
    async fn participants(&self, ctx: &Context<'_>) -> Result<Vec<Participant>> {
        let loader = ctx.data_unchecked::<UsersForEventLoader>();
        let participants = loader
            .load_one(self.slug.clone())
            .await
            .extend()?
            .unwrap_or_default();

        Ok(participants)
    }
}
