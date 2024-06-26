use crate::Result;
#[cfg(feature = "graphql")]
use crate::{
    loaders::{OrganizationLoader, UserLoader},
    Organization, User,
};
#[cfg(feature = "graphql")]
use async_graphql::{ComplexObject, Context, Enum, ResultExt, SimpleObject};
use chrono::{DateTime, Utc};
use context::UserRole;
use futures::stream::TryStreamExt;
use sqlx::{query, query_as, Executor};
use std::collections::HashMap;
use tracing::instrument;

/// A role that can be applied to an organizer
// TODO: consider switching to a bit flags permission implementation a la Discord
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, sqlx::Type)]
#[cfg_attr(feature = "graphql", derive(Enum))]
#[sqlx(rename_all = "lowercase", type_name = "organizer_role")]
pub enum Role {
    /// Has full permissions within the organization and event
    Director,
    /// An elevated user within the organization that change event and organization settings
    Manager,
    /// A normal user within the organization
    #[default]
    Organizer,
}

impl From<Role> for UserRole {
    fn from(role: Role) -> Self {
        match role {
            Role::Director => UserRole::Director,
            Role::Manager => UserRole::Manager,
            Role::Organizer => UserRole::Organizer,
        }
    }
}

/// Maps a user to an organization as an organizer
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "graphql", derive(SimpleObject))]
#[cfg_attr(feature = "graphql", graphql(complex))]
pub struct Organizer {
    /// The organization ID
    #[cfg_attr(feature = "graphql", graphql(skip))]
    pub organization_id: i32,
    /// The user ID
    #[cfg_attr(feature = "graphql", graphql(skip))]
    pub user_id: i32,
    /// The permissions the user has
    pub role: Role,
    /// When the mapping was created
    pub created_at: DateTime<Utc>,
    /// When the mapping was last updated
    pub updated_at: DateTime<Utc>,
}

#[cfg(feature = "graphql")]
#[ComplexObject]
impl Organizer {
    /// The organization the user is part of
    #[instrument(name = "Organizer::organization", skip_all, fields(%self.organization_id, %self.user_id))]
    async fn organization(&self, ctx: &Context<'_>) -> async_graphql::Result<Organization> {
        let loader = ctx.data_unchecked::<OrganizationLoader>();
        let organization = loader
            .load_one(self.organization_id)
            .await
            .extend()?
            .expect("organization must exist");

        Ok(organization)
    }

    /// The user that is part of the organization
    #[instrument(name = "Organizer::user", skip_all, fields(%self.organization_id, %self.user_id))]
    async fn user(&self, ctx: &Context<'_>) -> async_graphql::Result<User> {
        let loader = ctx.data_unchecked::<UserLoader>();
        let user = loader
            .load_one(self.user_id)
            .await
            .extend()?
            .expect("user must exist");

        Ok(user)
    }
}

impl Organizer {
    /// Load all the organizer info for a user, for use in dataloaders
    #[instrument(name = "Organizer::load_for_user", skip(db))]
    pub(crate) async fn load_for_user<'c, 'e, E>(
        user_ids: &[i32],
        db: E,
    ) -> Result<HashMap<i32, Vec<Organizer>>>
    where
        'c: 'e,
        E: 'e + Executor<'c, Database = sqlx::Postgres>,
    {
        let by_user_id = query_as!(
            Organizer,
            r#"
            SELECT organization_id, user_id, role as "role: Role", created_at, updated_at
            FROM organizers
            WHERE user_id = ANY($1)
            "#,
            user_ids,
        )
        .fetch(db)
        .try_fold(HashMap::new(), |mut map, organizer| async move {
            let entry: &mut Vec<Organizer> = map.entry(organizer.user_id).or_default();
            entry.push(organizer);
            Ok(map)
        })
        .await?;

        Ok(by_user_id)
    }

    /// Load all the organizer info for an organization, for use in dataloaders
    #[instrument(name = "Organizer::load_for_organization")]
    pub(crate) async fn load_for_organization<'c, 'e, E>(
        organization_ids: &[i32],
        db: E,
    ) -> Result<HashMap<i32, Vec<Organizer>>>
    where
        'c: 'e,
        E: 'e + Executor<'c, Database = sqlx::Postgres>,
    {
        let by_organization_id = query_as!(
            Organizer,
            r#"
            SELECT organization_id, user_id, role as "role: Role", created_at, updated_at
            FROM organizers
            WHERE organization_id = ANY($1)
            "#,
            organization_ids
        )
        .fetch(db)
        .try_fold(HashMap::new(), |mut map, organizer| async move {
            let entry: &mut Vec<Organizer> = map.entry(organizer.organization_id).or_default();
            entry.push(organizer);
            Ok(map)
        })
        .await?;

        Ok(by_organization_id)
    }

    /// Find an organizer entry
    #[instrument(name = "Organizer::find", skip(db))]
    pub async fn find<'c, 'e, E>(
        user_id: i32,
        organization_id: i32,
        db: E,
    ) -> Result<Option<Organizer>>
    where
        'c: 'e,
        E: 'e + Executor<'c, Database = sqlx::Postgres>,
    {
        let organizer = query_as!(
            Organizer,
            r#"
            SELECT organization_id, user_id, role as "role: Role", created_at, updated_at
            FROM organizers
            WHERE organization_id = $1 AND user_id = $2
            "#,
            organization_id,
            user_id
        )
        .fetch_optional(db)
        .await?;
        Ok(organizer)
    }

    /// Get all the organizations a user is part of
    #[instrument(name = "Organizer::for_user", skip(db))]
    pub async fn for_user<'c, 'e, E>(user_id: i32, db: E) -> Result<Vec<Organizer>>
    where
        'c: 'e,
        E: 'e + Executor<'c, Database = sqlx::Postgres>,
    {
        let organizers = query_as!(
            Organizer,
            r#"
            SELECT organization_id, user_id, role as "role: Role", created_at, updated_at
            FROM organizers
            WHERE user_id = $1
            "#,
            user_id,
        )
        .fetch_all(db)
        .await?;

        Ok(organizers)
    }

    /// Get all the users in an organization
    #[instrument(name = "Organizer::for_organization", skip(db))]
    pub async fn for_organization<'c, 'e, E>(organization_id: i32, db: E) -> Result<Vec<Organizer>>
    where
        'c: 'e,
        E: 'e + Executor<'c, Database = sqlx::Postgres>,
    {
        let organizers = query_as!(
            Organizer,
            r#"
            SELECT organization_id, user_id, role as "role: Role", created_at, updated_at
            FROM organizers
            WHERE organization_id = $1
            "#,
            organization_id,
        )
        .fetch_all(db)
        .await?;

        Ok(organizers)
    }

    /// Add a user to an organization
    #[instrument(name = "Organizer::add", skip(db))]
    pub async fn add<'c, 'e, E>(
        organization_id: i32,
        user_id: i32,
        role: Role,
        db: E,
    ) -> Result<Organizer>
    where
        'c: 'e,
        E: 'e + Executor<'c, Database = sqlx::Postgres>,
    {
        let organizer = query_as!(
            Organizer,
            r#"
            INSERT INTO organizers (organization_id, user_id, role) 
            VALUES ($1, $2, $3) 
            ON CONFLICT (organization_id, user_id) DO UPDATE SET role = excluded.role
            RETURNING organization_id, user_id, role as "role: Role", created_at, updated_at
            "#,
            organization_id,
            user_id,
            role as _,
        )
        .fetch_one(db)
        .await?;

        Ok(organizer)
    }

    /// Delete a user from an organization
    #[instrument(name = "Organizer::delete", skip(db))]
    pub async fn delete<'c, 'e, E>(organization_id: i32, user_id: i32, db: E) -> Result<()>
    where
        'c: 'e,
        E: 'e + Executor<'c, Database = sqlx::Postgres>,
    {
        query!(
            "DELETE FROM organizers WHERE organization_id = $1 AND user_id = $2",
            organization_id,
            user_id,
        )
        .execute(db)
        .await?;

        Ok(())
    }
}
