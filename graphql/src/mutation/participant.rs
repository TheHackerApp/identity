use super::UserError;
use crate::webhooks;
use async_graphql::{Context, InputObject, Object, Result, ResultExt, SimpleObject};
use database::{
    loaders::{EventLoader, UserLoader},
    Event, Participant, PgPool, User,
};
use tracing::instrument;

#[derive(Default)]
pub(crate) struct ParticipantMutation;

#[Object]
impl ParticipantMutation {
    /// Add a user to an event, as a participant
    #[instrument(name = "Mutation::add_user_to_event", skip(self, ctx))]
    async fn add_user_to_event(
        &self,
        ctx: &Context<'_>,
        input: AddUserToEventInput,
    ) -> Result<AddUserToEventResult> {
        let event_loader = ctx.data_unchecked::<EventLoader>();
        let Some(event) = event_loader.load_one(input.event).await.extend()? else {
            return Ok(UserError::new(&["event"], "event does not exist").into());
        };

        let user_loader = ctx.data_unchecked::<UserLoader>();
        let Some(user) = user_loader.load_one(input.user_id).await.extend()? else {
            return Ok(UserError::new(&["user_id"], "user does not exist").into());
        };

        let db = ctx.data_unchecked::<PgPool>();
        Participant::add(&event.slug, user.id, db).await.extend()?;

        let webhooks = ctx.data_unchecked::<webhooks::Client>();
        webhooks.on_participant_changed(user.id, &user.primary_email);

        Ok((user, event).into())
    }

    /// Remove a participant from an event
    #[instrument(name = "Mutation::remove_user_from_event", skip(self, ctx))]
    async fn remove_user_from_event(
        &self,
        ctx: &Context<'_>,
        input: RemoveUserFromEventInput,
    ) -> Result<RemoveUserFromEventResult> {
        let db = ctx.data_unchecked::<PgPool>();
        Participant::delete(&input.event, input.user_id, db)
            .await
            .extend()?;

        Ok((input.user_id, input.event).into())
    }
}

/// Input for adding a user to an event
#[derive(Debug, InputObject)]
struct AddUserToEventInput {
    /// The slug of the event to add the user to
    event: String,
    /// The ID of the user to add
    user_id: i32,
}

#[derive(Debug, SimpleObject)]
struct AddUserToEventResult {
    /// The user that was added to the event
    user: Option<User>,
    /// The event the user was added to
    event: Option<Event>,
    /// Errors that may have occurred while processing the action
    user_errors: Vec<UserError>,
}

impl From<(User, Event)> for AddUserToEventResult {
    fn from((user, event): (User, Event)) -> Self {
        Self {
            user: Some(user),
            event: Some(event),
            user_errors: Vec::with_capacity(0),
        }
    }
}

impl From<UserError> for AddUserToEventResult {
    fn from(user_error: UserError) -> Self {
        Self {
            user: None,
            event: None,
            user_errors: vec![user_error],
        }
    }
}

/// Input for removing a user from an event
#[derive(Debug, InputObject)]
struct RemoveUserFromEventInput {
    /// The slug of the event to remove the user from
    event: String,
    /// The ID of the user to remove
    user_id: i32,
}

#[derive(Debug, SimpleObject)]
struct RemoveUserFromEventResult {
    /// The ID of the user that was removed from the event
    removed_user_id: Option<i32>,
    /// The event the user was removed from
    event: Option<String>,
    /// Errors that may have occurred while processing the action
    user_errors: Vec<UserError>,
}

impl From<(i32, String)> for RemoveUserFromEventResult {
    fn from((user_id, event): (i32, String)) -> Self {
        Self {
            removed_user_id: Some(user_id),
            event: Some(event),
            user_errors: Vec::with_capacity(0),
        }
    }
}
