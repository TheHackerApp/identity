use super::{results, UserError};
use crate::loaders::UserLoader;
use async_graphql::{Context, InputObject, Object, Result, ResultExt};
use database::{Identity, PgPool, User};
use tracing::instrument;

results! {
    UpdateUserResult {
        /// The user
        user: User,
    }
    DeleteUserResult {
        /// The ID of the deleted user
        deleted_id: i32,
    }
}

#[derive(Default)]
pub(crate) struct UserMutation;

#[Object]
impl UserMutation {
    /// Update the details of a user
    #[instrument(name = "Mutation::update_user", skip(self, ctx))]
    async fn update_user(
        &self,
        ctx: &Context<'_>,
        input: UpdateUserInput,
    ) -> Result<UpdateUserResult> {
        let mut user_errors = Vec::new();

        if let Some(given_name) = &input.given_name {
            if given_name.is_empty() {
                user_errors.push(UserError::new(&["given_name"], "cannot be empty"));
            }
        }

        if let Some(family_name) = &input.family_name {
            if family_name.is_empty() {
                user_errors.push(UserError::new(&["family_name"], "cannot be empty"));
            }
        }

        if !user_errors.is_empty() {
            return Ok(user_errors.into());
        }

        let loader = ctx.data_unchecked::<UserLoader>();
        let mut user = match loader.load_one(input.id).await.extend()? {
            Some(u) => u,
            None => return Ok(UserError::new(&["id"], "user does not exist").into()),
        };

        let db = ctx.data_unchecked::<PgPool>();
        let identities = Identity::for_user(user.id, db).await.extend()?;
        if let Some(primary_email) = &input.primary_email {
            if !identities.iter().any(|i| &i.email == primary_email) {
                return Ok(UserError::new(
                    &["primary_email"],
                    "primary email must be linked to an identity",
                )
                .into());
            }
        }

        user.update()
            .override_given_name(input.given_name)
            .override_family_name(input.family_name)
            .override_primary_email(input.primary_email)
            .override_is_admin(input.is_admin)
            .save(db)
            .await
            .extend()?;

        Ok(user.into())
    }

    /// Delete a user
    #[instrument(name = "Mutation::delete_user", skip(self, ctx))]
    async fn delete_user(&self, ctx: &Context<'_>, id: i32) -> Result<DeleteUserResult> {
        let db = ctx.data::<PgPool>()?;
        User::delete(id, db).await.extend()?;

        Ok(id.into())
    }
}

/// Input fields for updating a user
#[derive(Debug, InputObject)]
struct UpdateUserInput {
    /// The ID of the user to update
    pub id: i32,
    /// The given/first name
    pub given_name: Option<String>,
    /// The family/last name
    pub family_name: Option<String>,
    /// The primary email as selected by the user
    pub primary_email: Option<String>,
    /// Whether the user is an administrator
    pub is_admin: Option<bool>,
}
