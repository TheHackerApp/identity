use super::{base::HasSessionState, InvalidSessionState, Mutable, SessionState};
use axum::{
    async_trait,
    extract::{FromRef, FromRequestParts},
    http::{request::Parts, StatusCode},
    response::{IntoResponse, Response},
};
use database::{PgPool, User};
use std::fmt::Debug;
use tracing::error;

/// Retrieve the current user from the session
#[derive(Debug)]
pub struct CurrentUser<T>
where
    T: HasSessionState,
{
    session: T,
    user: User,
}

impl<T> CurrentUser<T>
where
    T: HasSessionState,
{
    /// Get the raw user
    pub fn into_inner(self) -> User {
        self.user
    }
}

impl CurrentUser<Mutable> {
    /// Logout the current user
    pub fn logout(mut self) {
        self.session.state = SessionState::Unauthenticated
    }
}

impl<T> std::ops::Deref for CurrentUser<T>
where
    T: HasSessionState,
{
    type Target = User;

    fn deref(&self) -> &Self::Target {
        &self.user
    }
}

impl<T> std::ops::DerefMut for CurrentUser<T>
where
    T: HasSessionState,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.user
    }
}

#[async_trait]
impl<T, S> FromRequestParts<S> for CurrentUser<T>
where
    T: HasSessionState + FromRequestParts<S> + Send + Debug,
    <T as FromRequestParts<S>>::Rejection: Debug,
    S: Send + Sync,
    PgPool: FromRef<S>,
{
    type Rejection = CurrentUserRejection;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let session = T::from_request_parts(parts, state).await.unwrap();
        let db = PgPool::from_ref(state);

        let id = session
            .state()
            .id()
            .ok_or_else(|| InvalidSessionState::from(&session.state()))?;

        let user = User::find(id, &db)
            .await?
            .ok_or(CurrentUserRejection::UnknownUser(id))?;

        Ok(Self { user, session })
    }
}

#[derive(Debug)]
pub enum CurrentUserRejection {
    /// Propagate a session state error
    InvalidSessionState(InvalidSessionState),
    /// An unexpected database error
    Database(database::Error),
    /// The user in the session could not be found
    UnknownUser(i32),
}

impl IntoResponse for CurrentUserRejection {
    fn into_response(self) -> Response {
        match self {
            Self::InvalidSessionState(rejection) => rejection.into_response(),
            Self::Database(error) => {
                use std::error::Error;

                match error.source() {
                    Some(source) => error!(%error, %source, "unexpected database error"),
                    None => error!(%error, "unexpected database error"),
                }
                (StatusCode::INTERNAL_SERVER_ERROR, "internal error").into_response()
            }
            Self::UnknownUser(id) => {
                error!(%id, "user specified in session does not exist");
                (StatusCode::INTERNAL_SERVER_ERROR, "internal error").into_response()
            }
        }
    }
}

impl From<InvalidSessionState> for CurrentUserRejection {
    fn from(rejection: InvalidSessionState) -> Self {
        CurrentUserRejection::InvalidSessionState(rejection)
    }
}

impl From<database::Error> for CurrentUserRejection {
    fn from(error: database::Error) -> Self {
        CurrentUserRejection::Database(error)
    }
}
