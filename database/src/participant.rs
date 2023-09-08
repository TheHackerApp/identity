use crate::Result;
use chrono::{DateTime, Utc};
use sqlx::{query, query_as, PgPool};
use tracing::instrument;

/// Maps a user to an event as a participant
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Participant {
    /// The event slug
    pub event: String,
    /// The user ID
    pub user_id: i32,
    /// When the mapping was first created
    pub created_at: DateTime<Utc>,
    /// When the mapping was last updated
    pub updated_at: DateTime<Utc>,
}

impl Participant {
    /// Get all the events a user is participating in
    #[instrument(name = "Participant::for_user", skip(db))]
    pub async fn for_user(user_id: i32, db: &PgPool) -> Result<Vec<Participant>> {
        let participants = query_as!(
            Participant,
            "SELECT * FROM participants WHERE user_id = $1",
            user_id,
        )
        .fetch_all(db)
        .await?;

        Ok(participants)
    }

    /// Get all the users participating in an event
    #[instrument(name = "Participant::for_event", skip(db))]
    pub async fn for_event(event: &str, db: &PgPool) -> Result<Vec<Participant>> {
        let participants = query_as!(
            Participant,
            "SELECT * FROM participants WHERE event = $1",
            event,
        )
        .fetch_all(db)
        .await?;

        Ok(participants)
    }

    /// Map a user to an event
    #[instrument(name = "Participant::create", skip(db))]
    pub async fn create(event: &str, user_id: i32, db: &PgPool) -> Result<Participant> {
        let participant = query_as!(
            Participant,
            "INSERT INTO participants (event, user_id) VALUES ($1, $2) RETURNING *",
            event,
            user_id,
        )
        .fetch_one(db)
        .await?;

        Ok(participant)
    }

    /// Delete a user from an event
    #[instrument(name = "Participant::delete", skip(db))]
    pub async fn delete(event: &str, user_id: i32, db: &PgPool) -> Result<()> {
        query!(
            "DELETE FROM participants WHERE event = $1 AND user_id = $2",
            event,
            user_id,
        )
        .execute(db)
        .await?;

        Ok(())
    }
}
