use crate::Result;
#[cfg(feature = "graphql")]
use crate::{
    loaders::{EventLoader, UserLoader},
    Event, User,
};
#[cfg(feature = "graphql")]
use async_graphql::{ComplexObject, Context, ResultExt, SimpleObject};
use chrono::{DateTime, Utc};
use futures::stream::TryStreamExt;
use sqlx::{query, query_as, Executor};
use std::collections::HashMap;
use tracing::instrument;

/// Maps a user to an event as a participant
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "graphql", derive(SimpleObject))]
#[cfg_attr(feature = "graphql", graphql(complex))]
pub struct Participant {
    /// The event slug
    #[cfg_attr(feature = "graphql", graphql(skip))]
    pub event: String,
    /// The user ID
    #[cfg_attr(feature = "graphql", graphql(skip))]
    pub user_id: i32,
    /// When the mapping was first created
    pub created_at: DateTime<Utc>,
    /// When the mapping was last updated
    pub updated_at: DateTime<Utc>,
}

#[cfg(feature = "graphql")]
#[ComplexObject]
impl Participant {
    /// The event the user is participating in
    #[instrument(name = "Participant::event", skip_all, fields(%self.event, %self.user_id))]
    async fn event(&self, ctx: &Context<'_>) -> async_graphql::Result<Event> {
        let loader = ctx.data_unchecked::<EventLoader>();
        let event = loader
            .load_one(self.event.clone())
            .await
            .extend()?
            .expect("event must exist");

        Ok(event)
    }

    /// The user associated with the event
    #[instrument(name = "Participant::user", skip_all, fields(%self.event, %self.user_id))]
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

impl Participant {
    /// Load all the event slugs for a user, for use in dataloaders
    #[instrument(name = "Participant::load_for_user", skip(db))]
    pub(crate) async fn load_for_user<'c, 'e, E>(
        user_ids: &[i32],
        db: E,
    ) -> Result<HashMap<i32, Vec<Participant>>>
    where
        'c: 'e,
        E: 'e + Executor<'c, Database = sqlx::Postgres>,
    {
        let by_user_id = query_as!(
            Participant,
            "SELECT * FROM participants WHERE user_id = ANY($1)",
            user_ids
        )
        .fetch(db)
        .try_fold(HashMap::new(), |mut map, participant| async move {
            let entry: &mut Vec<Participant> = map.entry(participant.user_id).or_default();
            entry.push(participant);
            Ok(map)
        })
        .await?;

        Ok(by_user_id)
    }

    /// Load all the participants for an event, for use in dataloaders
    #[instrument(name = "Participant::load_for_event", skip(db))]
    pub(crate) async fn load_for_event<'c, 'e, E>(
        slugs: &[String],
        db: E,
    ) -> Result<HashMap<String, Vec<Participant>>>
    where
        'c: 'e,
        E: 'e + Executor<'c, Database = sqlx::Postgres>,
    {
        let by_event = query_as!(
            Participant,
            "SELECT * FROM participants WHERE event = ANY($1)",
            slugs
        )
        .fetch(db)
        .try_fold(HashMap::new(), |mut map, participant| async move {
            let entry: &mut Vec<Participant> = map.entry(participant.event.clone()).or_default();
            entry.push(participant);
            Ok(map)
        })
        .await?;

        Ok(by_event)
    }

    /// Get all the events a user is participating in
    #[instrument(name = "Participant::for_user", skip(db))]
    pub async fn for_user<'c, 'e, E>(user_id: i32, db: E) -> Result<Vec<Participant>>
    where
        'c: 'e,
        E: 'e + Executor<'c, Database = sqlx::Postgres>,
    {
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
    pub async fn for_event<'c, 'e, E>(event: &str, db: E) -> Result<Vec<Participant>>
    where
        'c: 'e,
        E: 'e + Executor<'c, Database = sqlx::Postgres>,
    {
        let participants = query_as!(
            Participant,
            "SELECT * FROM participants WHERE event = $1",
            event,
        )
        .fetch_all(db)
        .await?;

        Ok(participants)
    }

    /// Add a user to an event
    #[instrument(name = "Participant::add", skip(db))]
    pub async fn add<'c, 'e, E>(event: &str, user_id: i32, db: E) -> Result<Participant>
    where
        'c: 'e,
        E: 'e + Executor<'c, Database = sqlx::Postgres>,
    {
        // The updated_at column needs to be explicitly set so rows are returned
        let participant = query_as!(
            Participant,
            r#"
            INSERT INTO participants (event, user_id) 
            VALUES ($1, $2) 
            ON CONFLICT (event, user_id) DO UPDATE SET updated_at = now()
            RETURNING *
            "#,
            event,
            user_id,
        )
        .fetch_one(db)
        .await?;

        Ok(participant)
    }

    /// Delete a user from an event
    #[instrument(name = "Participant::delete", skip(db))]
    pub async fn delete<'c, 'e, E>(event: &str, user_id: i32, db: E) -> Result<()>
    where
        'c: 'e,
        E: 'e + Executor<'c, Database = sqlx::Postgres>,
    {
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
