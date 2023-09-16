use super::UserError;
use async_graphql::{Context, InputObject, Object, Result, ResultExt, SimpleObject};
use database::{Organizer, PgPool, User};
use tracing::instrument;

#[derive(Default)]
pub(crate) struct OrganizerMutation;

#[Object]
impl OrganizerMutation {
    /// Add a user to an organization
    #[instrument(name = "Mutation::add_user_to_organization", skip(self, ctx))]
    async fn add_user_to_organization(
        &self,
        ctx: &Context<'_>,
        input: AddUserToOrganizationInput,
    ) -> Result<AddUserToOrganizationResult> {
        let db = ctx.data::<PgPool>()?;

        // TODO: ensure organization exists

        let Some(user) = User::find(input.user_id, db).await.extend()? else {
            return Ok(UserError::new(&["user_id"], "user does not exist").into());
        };

        Organizer::create(input.organization_id, user.id, db)
            .await
            .extend()?;

        Ok((user, input.organization_id).into())
    }

    /// Remove a user from an organization
    #[instrument(name = "Mutation::remove_user_from_organization", skip(self, ctx))]
    async fn remove_user_from_organization(
        &self,
        ctx: &Context<'_>,
        input: RemoveUserFromOrganizationInput,
    ) -> Result<RemoveUserFromOrganizationResult> {
        let db = ctx.data::<PgPool>()?;
        Organizer::delete(input.organization_id, input.user_id, db)
            .await
            .extend()?;

        Ok((input.user_id, input.organization_id).into())
    }
}

/// Input for adding a user to an organization
#[derive(Debug, InputObject)]
struct AddUserToOrganizationInput {
    /// The ID of the organization to add the user to
    organization_id: i32,
    /// The ID of the user to add
    user_id: i32,
}

#[derive(Debug, SimpleObject)]
struct AddUserToOrganizationResult {
    /// The user that was added to the organization
    user: Option<User>,
    /// The organization the user was added to
    organization: Option<i32>,
    /// Errors that may have occurred while processing the action
    user_errors: Vec<UserError>,
}

impl From<(User, i32)> for AddUserToOrganizationResult {
    fn from((user, organization): (User, i32)) -> Self {
        Self {
            user: Some(user),
            organization: Some(organization),
            user_errors: Vec::with_capacity(0),
        }
    }
}

impl From<UserError> for AddUserToOrganizationResult {
    fn from(user_error: UserError) -> Self {
        Self {
            user: None,
            organization: None,
            user_errors: vec![user_error],
        }
    }
}

/// Input for removing a user from an organization
#[derive(Debug, InputObject)]
struct RemoveUserFromOrganizationInput {
    /// The ID of the organization to remove the user from
    organization_id: i32,
    /// The ID of the user to remove
    user_id: i32,
}

#[derive(Debug, SimpleObject)]
struct RemoveUserFromOrganizationResult {
    /// The ID of the user that was removed from the organization
    removed_user_id: Option<i32>,
    /// The organization the user was removed from
    organization: Option<i32>,
    /// Errors that may have occurred while processing the action
    user_errors: Vec<UserError>,
}

impl From<(i32, i32)> for RemoveUserFromOrganizationResult {
    fn from((user_id, organization): (i32, i32)) -> Self {
        Self {
            removed_user_id: Some(user_id),
            organization: Some(organization),
            user_errors: Vec::with_capacity(0),
        }
    }
}
