use super::{results, UserError};
use async_graphql::{Context, InputObject, Object, Result, ResultExt};
use database::{Identity, PgPool};
use tracing::instrument;

results! {
    UnlinkIdentityResult {
        /// The provider that was unlinked
        unlinked_provider: String,
    }
}

#[derive(Default)]
pub(crate) struct IdentityMutation;

#[Object]
impl IdentityMutation {
    // TODO: add linking flow

    /// Unlink an authentication provider identity from a user
    #[instrument(name = "Mutation::unlink_identity", skip(self, ctx))]
    async fn unlink_identity(
        &self,
        ctx: &Context<'_>,
        input: UnlinkIdentityInput,
    ) -> Result<UnlinkIdentityResult> {
        let db = ctx.data::<PgPool>()?;

        let identities = Identity::for_user(input.user_id, db).await.extend()?;
        if identities.len() == 1 {
            return Ok(UserError::new(&["provider"], "must have one identity linked").into());
        }

        Identity::unlink(&input.provider, input.user_id, db)
            .await
            .extend()?;

        Ok(input.provider.into())
    }
}

/// Input for unlinking a user's authentication provider identity
#[derive(Debug, InputObject)]
struct UnlinkIdentityInput {
    /// The ID of the user to perform the unlinking on
    user_id: i32,
    /// THe provider to unlink
    provider: String,
}