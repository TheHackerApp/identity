use super::{base::HasSessionState, Immutable, InvalidSessionState, Mutable, SessionState};
use crate::{session::RegistrationNeededState, state::FrontendUrl};
use axum::{
    async_trait,
    extract::{FromRef, FromRequestParts},
    http::request::Parts,
};
use std::fmt::Debug;
use tracing::debug;

/// A session where the user needs to complete their registration.
///
/// Registration needed sessions can only become authenticated once a user is created.
#[derive(Debug)]
pub struct RegistrationNeededSession<T>(T)
where
    T: HasSessionState;

impl RegistrationNeededSession<Mutable> {
    /// Make the current session authenticated for the newly created user
    pub fn into_authenticated(mut self, id: i32) {
        self.0.state = SessionState::authenticated(id)
    }
}

impl<T> std::ops::Deref for RegistrationNeededSession<T>
where
    T: HasSessionState,
{
    type Target = RegistrationNeededState;

    fn deref(&self) -> &Self::Target {
        // We know this condition holds due to the FromRequestParts implementation
        match self.0.state() {
            SessionState::RegistrationNeeded(state) => state,
            _ => unreachable!(),
        }
    }
}

#[async_trait]
impl<T, S> FromRequestParts<S> for RegistrationNeededSession<T>
where
    T: HasSessionState + FromRequestParts<S> + Debug,
    <T as FromRequestParts<S>>::Rejection: Debug,
    S: Send + Sync,
    FrontendUrl: FromRef<S>,
    RegistrationNeededSession<T>: From<T>,
{
    type Rejection = InvalidSessionState;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let session = T::from_request_parts(parts, state).await.unwrap();

        match session.state() {
            SessionState::RegistrationNeeded(_) => Ok(session.into()),
            session => {
                debug!("invalid session state, expected registration needed");
                Err(InvalidSessionState::from(state, session))
            }
        }
    }
}

impl From<Mutable> for RegistrationNeededSession<Mutable> {
    fn from(session: Mutable) -> Self {
        Self(session)
    }
}

impl From<Immutable> for RegistrationNeededSession<Immutable> {
    fn from(session: Immutable) -> Self {
        Self(session)
    }
}
