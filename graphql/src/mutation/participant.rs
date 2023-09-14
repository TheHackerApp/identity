use super::{results, UserError};
use async_graphql::{Context, InputObject, Object, Result, ResultExt};
use database::{Participant, PgPool, User};
use tracing::instrument;

results! {
    AddUserToEventResult {
        /// The user that was added to the event
        user: User
    }
    DeleteUserFromEventResult {
        /// The ID of the user that was removed from the event
        deleted_user_id: i32,
    }
}

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
        let db = ctx.data::<PgPool>()?;

        // TODO: ensure event exists

        let Some(user) = User::find(input.user_id, db).await.extend()? else {
            return Ok(UserError::new(&["user_id"], "user does not exist").into());
        };

        Participant::create(&input.event, input.user_id, db)
            .await
            .extend()?;

        Ok(user.into())
    }

    /// Remove a participant from an event
    #[instrument(name = "Mutation::delete_user_from_event", skip(self, ctx))]
    async fn delete_user_from_event(
        &self,
        ctx: &Context<'_>,
        input: DeleteUserFromEventInput,
    ) -> Result<DeleteUserFromEventResult> {
        let db = ctx.data::<PgPool>()?;
        Participant::delete(&input.event, input.user_id, db)
            .await
            .extend()?;

        Ok(input.user_id.into())
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

/// Input for removing a user from an event
#[derive(Debug, InputObject)]
struct DeleteUserFromEventInput {
    /// The slug of the event to remove the user from
    event: String,
    /// The ID of the user to remove
    user_id: i32,
}
