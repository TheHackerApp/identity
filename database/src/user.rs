#[cfg(feature = "graphql")]
use crate::Identity;
use crate::Result;
#[cfg(feature = "graphql")]
use async_graphql::{ComplexObject, Context, ResultExt};
use chrono::{DateTime, Utc};
use sqlx::{query, query_as, PgPool, QueryBuilder};
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
    /// Get a user by it's ID
    #[instrument(name = "User::find", skip(db))]
    pub async fn find(id: i32, db: &PgPool) -> Result<Option<User>> {
        let user = query_as!(User, "SELECT * FROM users WHERE id = $1", id)
            .fetch_optional(db)
            .await?;
        Ok(user)
    }

    /// Get a user by it's primary email
    #[instrument(name = "User::find_by_primary_email", skip(db))]
    pub async fn find_by_primary_email(email: &str, db: &PgPool) -> Result<Option<User>> {
        let user = query_as!(User, "SELECT * FROM users WHERE primary_email = $1", email)
            .fetch_optional(db)
            .await?;
        Ok(user)
    }

    /// Create a new user
    #[instrument(name = "User::create", skip(db))]
    pub async fn create(
        given_name: &str,
        family_name: &str,
        primary_email: &str,
        db: &PgPool,
    ) -> Result<User> {
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
    pub async fn delete(id: i32, db: &PgPool) -> Result<()> {
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
    #[instrument(name = "User::identities", skip_all)]
    async fn identities(&self, ctx: &Context<'_>) -> async_graphql::Result<Vec<Identity>> {
        let db = ctx.data::<PgPool>()?;
        let identities = Identity::for_user(self.id, db).await.extend()?;

        Ok(identities)
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
    pub async fn save(self, db: &PgPool) -> Result<()> {
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
