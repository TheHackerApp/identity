use crate::Result;
#[cfg(feature = "graphql")]
use crate::{
    loaders::{EventsForUserLoader, IdentitiesForUserLoader, OrganizationsForUserLoader},
    Identity, Organizer, Participant,
};
#[cfg(feature = "graphql")]
use async_graphql::{ComplexObject, Context, ResultExt};
use chrono::{DateTime, Utc};
use futures::stream::TryStreamExt;
use sqlx::{query, query_as, Executor, QueryBuilder};
use std::collections::HashMap;
use tracing::instrument;

/// A user of the service
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
#[cfg_attr(feature = "graphql", graphql(complex))]
pub struct User {
    /// A unique ID
    pub id: i32,
    /// The given/first name
    pub given_name: String,
    /// The family/last name
    pub family_name: String,
    /// The primary email as selected by the user
    pub primary_email: String,
    /// Whether the user is an administrator
    pub is_admin: bool,
    /// When the user was first created
    pub created_at: DateTime<Utc>,
    /// When the user was last updated
    pub updated_at: DateTime<Utc>,
}

impl User {
    /// Load all the users by their IDs, for use in dataloaders
    #[instrument(name = "User::load", skip(db))]
    pub(crate) async fn load<'c, 'e, E>(ids: &[i32], db: E) -> Result<HashMap<i32, User>>
    where
        'c: 'e,
        E: 'e + Executor<'c, Database = sqlx::Postgres>,
    {
        let by_id = query_as!(User, "SELECT * FROM users WHERE id = ANY($1)", ids)
            .fetch(db)
            .map_ok(|user| (user.id, user))
            .try_collect()
            .await?;
        Ok(by_id)
    }

    /// Load all the users by their primary emails, for use in dataloaders
    #[instrument(name = "User::load_by_primary_email", skip(db))]
    pub(crate) async fn load_by_primary_email<'c, 'e, E>(
        emails: &[String],
        db: E,
    ) -> Result<HashMap<String, User>>
    where
        'c: 'e,
        E: 'e + Executor<'c, Database = sqlx::Postgres>,
    {
        let by_primary_email = query_as!(
            User,
            "SELECT * FROM users WHERE primary_email = ANY($1)",
            emails
        )
        .fetch(db)
        .map_ok(|user| (user.primary_email.clone(), user))
        .try_collect()
        .await?;
        Ok(by_primary_email)
    }

    /// Check if a user exists
    #[instrument(name = "User::exists", skip(db))]
    pub async fn exists<'c, 'e, E>(id: i32, db: E) -> Result<bool>
    where
        'c: 'e,
        E: 'e + Executor<'c, Database = sqlx::Postgres>,
    {
        let result = query!("SELECT exists(SELECT 1 FROM users WHERE id = $1)", id)
            .fetch_one(db)
            .await?;

        Ok(result.exists.unwrap_or_default())
    }

    /// Get a user by it's ID
    #[instrument(name = "User::find", skip(db))]
    pub async fn find<'c, 'e, E>(id: i32, db: E) -> Result<Option<User>>
    where
        'c: 'e,
        E: 'e + Executor<'c, Database = sqlx::Postgres>,
    {
        let user = query_as!(User, "SELECT * FROM users WHERE id = $1", id)
            .fetch_optional(db)
            .await?;
        Ok(user)
    }

    /// Get a user by it's primary email
    #[instrument(name = "User::find_by_primary_email", skip(db))]
    pub async fn find_by_primary_email<'c, 'e, E>(email: &str, db: E) -> Result<Option<User>>
    where
        'c: 'e,
        E: 'e + Executor<'c, Database = sqlx::Postgres>,
    {
        let user = query_as!(User, "SELECT * FROM users WHERE primary_email = $1", email)
            .fetch_optional(db)
            .await?;
        Ok(user)
    }

    /// Create a new user
    #[instrument(name = "User::create", skip(db))]
    pub async fn create<'c, 'e, E>(
        given_name: &str,
        family_name: &str,
        primary_email: &str,
        db: E,
    ) -> Result<User>
    where
        'c: 'e,
        E: 'e + Executor<'c, Database = sqlx::Postgres>,
    {
        let user = query_as!(
            User,
            r#"
            INSERT INTO users (given_name, family_name, primary_email) 
            VALUES ($1, $2, $3) RETURNING *
            "#,
            given_name,
            family_name,
            primary_email,
        )
        .fetch_one(db)
        .await?;
        Ok(user)
    }

    /// Update the fields of a user
    pub fn update(&mut self) -> UserUpdater<'_> {
        UserUpdater::new(self)
    }

    /// Delete a user by it's ID
    #[instrument(name = "User::delete", skip(db))]
    pub async fn delete<'c, 'e, E>(id: i32, db: E) -> Result<()>
    where
        'c: 'e,
        E: 'e + Executor<'c, Database = sqlx::Postgres>,
    {
        query!("DELETE FROM users WHERE id = $1", id)
            .execute(db)
            .await?;

        Ok(())
    }
}

#[cfg(feature = "graphql")]
#[ComplexObject]
impl User {
    /// The identities the user can login with
    #[instrument(name = "User::identities", skip_all, fields(%self.id))]
    async fn identities(&self, ctx: &Context<'_>) -> async_graphql::Result<Vec<Identity>> {
        let loader = ctx.data_unchecked::<IdentitiesForUserLoader>();
        let identities = loader.load_one(self.id).await.extend()?.unwrap_or_default();

        Ok(identities)
    }

    /// The organizations the user is part of
    #[instrument(name = "User::organizations", skip_all, fields(%self.id))]
    async fn organizations(&self, ctx: &Context<'_>) -> async_graphql::Result<Vec<Organizer>> {
        let loader = ctx.data_unchecked::<OrganizationsForUserLoader>();
        let organizations = loader.load_one(self.id).await.extend()?.unwrap_or_default();

        Ok(organizations)
    }

    /// The events the user has joined
    #[instrument(name = "User::events", skip_all, fields(%self.id))]
    async fn events(&self, ctx: &Context<'_>) -> async_graphql::Result<Vec<Participant>> {
        let loader = ctx.data_unchecked::<EventsForUserLoader>();
        let events = loader.load_one(self.id).await.extend()?.unwrap_or_default();

        Ok(events)
    }
}

/// Handles updating individual fields of the user
pub struct UserUpdater<'u> {
    user: &'u mut User,
    given_name: Option<String>,
    family_name: Option<String>,
    primary_email: Option<String>,
    is_admin: Option<bool>,
}

impl<'u> UserUpdater<'u> {
    fn new(user: &'u mut User) -> UserUpdater<'u> {
        Self {
            user,
            given_name: None,
            family_name: None,
            primary_email: None,
            is_admin: None,
        }
    }

    /// Update the given name
    pub fn given_name(mut self, given_name: String) -> UserUpdater<'u> {
        self.given_name = Some(given_name);
        self
    }

    /// Directly set the given name
    pub fn override_given_name(mut self, given_name: Option<String>) -> UserUpdater<'u> {
        self.given_name = given_name;
        self
    }

    /// Update the family name
    pub fn family_name(mut self, family_name: String) -> UserUpdater<'u> {
        self.family_name = Some(family_name);
        self
    }

    /// Directly set the family name
    pub fn override_family_name(mut self, family_name: Option<String>) -> UserUpdater<'u> {
        self.family_name = family_name;
        self
    }

    /// Update the primary email
    pub fn primary_email(mut self, primary_email: String) -> UserUpdater<'u> {
        self.primary_email = Some(primary_email);
        self
    }

    /// Directly set the primary email
    pub fn override_primary_email(mut self, primary_email: Option<String>) -> UserUpdater<'u> {
        self.primary_email = primary_email;
        self
    }

    /// Update whether the user is an admin
    #[allow(clippy::wrong_self_convention)]
    pub fn is_admin(mut self, is_admin: bool) -> UserUpdater<'u> {
        self.is_admin = Some(is_admin);
        self
    }

    /// Directly set whether the user is an admin
    pub fn override_is_admin(mut self, is_admin: Option<bool>) -> UserUpdater<'u> {
        self.is_admin = is_admin;
        self
    }

    /// Perform the update
    #[instrument(name = "User::update", skip_all, fields(self.id = %self.user.id))]
    pub async fn save<'c, 'e, E>(self, db: E) -> Result<()>
    where
        'c: 'e,
        E: 'e + Executor<'c, Database = sqlx::Postgres>,
    {
        if self.given_name.is_none() && self.family_name.is_none() && self.primary_email.is_none() {
            // nothing was changed
            return Ok(());
        }

        let mut builder = QueryBuilder::new("UPDATE users SET ");
        let mut separated = builder.separated(", ");

        if let Some(given_name) = &self.given_name {
            separated.push("given_name = ");
            separated.push_bind_unseparated(given_name);
        }

        if let Some(family_name) = &self.family_name {
            separated.push("family_name = ");
            separated.push_bind_unseparated(family_name);
        }

        if let Some(primary_email) = &self.primary_email {
            separated.push("primary_email = ");
            separated.push_bind_unseparated(primary_email);
        }

        builder.push("WHERE id = ");
        builder.push_bind(self.user.id);
        builder.build().execute(db).await?;

        if let Some(given_name) = self.given_name {
            self.user.given_name = given_name;
        }

        if let Some(family_name) = self.family_name {
            self.user.family_name = family_name;
        }

        if let Some(primary_email) = self.primary_email {
            self.user.primary_email = primary_email;
        }

        Ok(())
    }
}
