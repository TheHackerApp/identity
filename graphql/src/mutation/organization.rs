use super::{results, validators, UserError};
use async_graphql::{Context, InputObject, MaybeUndefined, Object, Result, ResultExt};
use database::{loaders::OrganizationLoader, Organization, PgPool, User};
use tracing::instrument;

results! {
    CreateOrganizationResult {
        /// The created organization
        organization: Organization,
    }
    UpdateOrganizationResult {
        /// The organization
        organization: Organization,
    }
    TransferOrganizationOwnershipResult {
        /// The organization
        organization: Organization,
    }
    DeleteOrganizationResult {
        /// The ID of the deleted organization
        deleted_id: i32,
    }
}

#[derive(Default)]
pub(crate) struct OrganizationMutation;

#[Object]
impl OrganizationMutation {
    /// Add a new organization
    #[instrument(name = "Mutation::create_organization", skip(self, ctx))]
    async fn create_organization(
        &self,
        ctx: &Context<'_>,
        input: CreateOrganizationInput,
    ) -> Result<CreateOrganizationResult> {
        if input.name.is_empty() {
            return Ok(UserError::new(&["name"], "cannot be empty").into());
        }

        let db = ctx.data_unchecked::<PgPool>();

        if !User::exists(input.owner_id, db).await.extend()? {
            return Ok(UserError::new(&["owner_id"], "owner does not exist").into());
        }

        let organization = Organization::create(&input.name, input.owner_id, db)
            .await
            .extend()?;

        Ok(organization.into())
    }

    /// Update the details of an organization
    #[instrument(name = "Mutation::update_organization", skip(self, ctx))]
    async fn update_organization(
        &self,
        ctx: &Context<'_>,
        input: UpdateOrganizationInput,
    ) -> Result<UpdateOrganizationResult> {
        let mut user_errors = Vec::new();

        if let Some(name) = &input.name {
            if name.is_empty() {
                user_errors.push(UserError::new(&["name"], "cannot be empty"));
            }
        }

        if let MaybeUndefined::Value(logo) = &input.logo {
            if logo.is_empty() {
                user_errors.push(UserError::new(&["logo"], "cannot be empty"));
            }
            if !validators::url(&logo) {
                user_errors.push(UserError::new(&["logo"], "must be a URL"));
            }
        }

        if let MaybeUndefined::Value(website) = &input.website {
            if website.is_empty() {
                user_errors.push(UserError::new(&["website"], "cannot be empty"));
            }
            if !validators::url(&website) {
                user_errors.push(UserError::new(&["website"], "must be a URL"));
            }
        }

        if !user_errors.is_empty() {
            return Ok(user_errors.into());
        }

        let loader = ctx.data_unchecked::<OrganizationLoader>();
        let Some(mut organization) = loader.load_one(input.id).await.extend()? else {
            return Ok(UserError::new(&["id"], "organization does not exist").into());
        };

        let db = ctx.data_unchecked::<PgPool>();
        organization
            .update()
            .override_name(input.name)
            .override_logo(input.logo.into())
            .override_website(input.website.into())
            .save(db)
            .await
            .extend()?;

        Ok(organization.into())
    }

    /// Transfer the ownership of the organization to a different user
    #[instrument(name = "Mutation::transfer_organization_ownership", skip(self, ctx))]
    async fn transfer_organization_ownership(
        &self,
        ctx: &Context<'_>,
        input: TransferOrganizationOwnershipInput,
    ) -> Result<TransferOrganizationOwnershipResult> {
        let db = ctx.data_unchecked::<PgPool>();
        if !User::exists(input.new_owner_id, db).await.extend()? {
            return Ok(UserError::new(&["new_owner_id"], "new owner does not exist").into());
        }

        let organization_loader = ctx.data_unchecked::<OrganizationLoader>();
        let Some(mut organization) = organization_loader.load_one(input.id).await.extend()? else {
            return Ok(UserError::new(&["id"], "organization does not exist").into());
        };

        let db = ctx.data_unchecked::<PgPool>();
        organization
            .update()
            .owner(input.new_owner_id)
            .save(db)
            .await
            .extend()?;

        Ok(organization.into())
    }

    /// Delete an organization
    #[instrument(name = "Mutation::delete_organization", skip(self, ctx))]
    async fn delete_organization(
        &self,
        ctx: &Context<'_>,
        id: i32,
    ) -> Result<DeleteOrganizationResult> {
        let db = ctx.data::<PgPool>()?;
        Organization::delete(id, db).await.extend()?;

        Ok(id.into())
    }
}

/// Input fields for creating an organization
#[derive(Debug, InputObject)]
struct CreateOrganizationInput {
    /// The display name
    name: String,
    /// Who owns the organization
    owner_id: i32,
}

/// Input fields for updating an organization
#[derive(Debug, InputObject)]
struct UpdateOrganizationInput {
    /// The ID of the organization to update
    id: i32,
    /// The display name
    name: Option<String>,
    /// The URL of the organization's logo
    logo: MaybeUndefined<String>,
    /// The URL of the organization's website
    website: MaybeUndefined<String>,
}

/// Input fields for transferring the ownership of an organization
#[derive(Debug, InputObject)]
struct TransferOrganizationOwnershipInput {
    /// The ID of the organization to transfer ownership of
    id: i32,
    /// The ID of the new organization owner
    new_owner_id: i32,
}
