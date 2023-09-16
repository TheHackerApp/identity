use super::UserError;
use crate::loaders::UserLoader;
use async_graphql::{Context, InputObject, Object, Result, ResultExt, SimpleObject};
use database::{Participant, PgPool, User};
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
        // TODO: ensure event exists

        let loader = ctx.data_unchecked::<UserLoader>();
        let Some(user) = loader.load_one(input.user_id).await.extend()? else {
            return Ok(UserError::new(&["user_id"], "user does not exist").into());
        };

        let db = ctx.data_unchecked::<PgPool>();
        Participant::create(&input.event, input.user_id, db)
            .await
            .extend()?;

        Ok((user, input.event).into())
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
    event: Option<String>,
    /// Errors that may have occurred while processing the action
    user_errors: Vec<UserError>,
}

impl From<(User, String)> for AddUserToEventResult {
    fn from((user, event): (User, String)) -> Self {
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
