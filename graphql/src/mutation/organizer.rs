use super::{results, UserError};
use async_graphql::{Context, InputObject, Object, Result, ResultExt};
use database::{Organizer, PgPool, User};
use tracing::instrument;

results! {
    AddUserToOrganizationResult {
        /// The user that was added to the organization
        user: User,
    }
    DeleteUserFromOrganizationResult {
        /// The ID of the user that was removed from the organization
        deleted_user_id: i32,
    }
}

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

        Ok(user.into())
    }

    /// Remove a user from an organization
    #[instrument(name = "Mutation::delete_user_from_organization", skip(self, ctx))]
    async fn delete_user_from_organization(
        &self,
        ctx: &Context<'_>,
        input: DeleteUserFromOrganizationInput,
    ) -> Result<DeleteUserFromOrganizationResult> {
        let db = ctx.data::<PgPool>()?;
        Organizer::delete(input.organization_id, input.user_id, db)
            .await
            .extend()?;

        Ok(input.user_id.into())
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

/// Input for removing a user from an organization
#[derive(Debug, InputObject)]
struct DeleteUserFromOrganizationInput {
    /// The ID of the organization to remove the user from
    organization_id: i32,
    /// The ID of the user to remove
    user_id: i32,
}
