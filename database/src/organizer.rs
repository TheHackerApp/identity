use crate::Result;
use chrono::{DateTime, Utc};
use futures::stream::TryStreamExt;
use sqlx::{query, query_as, PgPool};
use std::collections::HashMap;
use tracing::instrument;

/// Maps a user to an organization as an organizer
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Organizer {
    /// The organization ID
    pub organization_id: i32,
    /// The user ID
    pub user_id: i32,
    /// When the mapping was created
    pub created_at: DateTime<Utc>,
    /// When the mapping was last updated
    pub updated_at: DateTime<Utc>,
}

impl Organizer {
    /// Load all the organization IDs for a user, for use in dataloaders
    pub(crate) async fn load_for_user(
        user_ids: &[i32],
        db: &PgPool,
    ) -> Result<HashMap<i32, Vec<i32>>> {
        let by_user_id = query_as!(
            Organizer,
            "SELECT * FROM organizers WHERE user_id = ANY($1)",
            user_ids
        )
        .fetch(db)
        .try_fold(HashMap::new(), |mut map, organizer| async move {
            let entry: &mut Vec<i32> = map.entry(organizer.user_id).or_default();
            entry.push(organizer.organization_id);
            Ok(map)
        })
        .await?;
        Ok(by_user_id)
    }

    /// Get all the organizations a user is part of
    #[instrument(name = "Organizer::for_user", skip(db))]
    pub async fn for_user(user_id: i32, db: &PgPool) -> Result<Vec<Organizer>> {
        let organizers = query_as!(
            Organizer,
            "SELECT * FROM organizers WHERE user_id = $1",
            user_id,
        )
        .fetch_all(db)
        .await?;

        Ok(organizers)
    }

    /// Get all the users in an organization
    #[instrument(name = "Organizer::for_organization", skip(db))]
    pub async fn for_organization(organization_id: i32, db: &PgPool) -> Result<Vec<Organizer>> {
        let organizers = query_as!(
            Organizer,
            "SELECT * FROM organizers WHERE organization_id = $1",
            organization_id,
        )
        .fetch_all(db)
        .await?;

        Ok(organizers)
    }

    /// Add a user to an organization
    #[instrument(name = "Organizer::create", skip(db))]
    pub async fn create(organization_id: i32, user_id: i32, db: &PgPool) -> Result<Organizer> {
        let organizer = query_as!(
            Organizer,
            "INSERT INTO organizers (organization_id, user_id) VALUES ($1, $2) RETURNING *",
            organization_id,
            user_id,
        )
        .fetch_one(db)
        .await?;

        Ok(organizer)
    }

    /// Delete a user from an organization
    #[instrument(name = "Organizer::delete", skip(db))]
    pub async fn delete(organization_id: i32, user_id: i32, db: &PgPool) -> Result<()> {
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
