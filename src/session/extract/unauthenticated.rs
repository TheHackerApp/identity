use super::{base::HasSessionState, Immutable, InvalidSessionState, Mutable, SessionState};
use axum::{async_trait, extract::FromRequestParts, http::request::Parts};
use std::fmt::Debug;
use tracing::debug;

/// An authenticated session that can initiate an OAuth2 login flow
#[derive(Debug)]
pub struct UnauthenticatedSession<T>(T)
where
    T: HasSessionState;

impl UnauthenticatedSession<Mutable> {
    /// Convert the current session to an in-flight OAuth2 session
    pub fn into_oauth(mut self, provider: String, state: String) {
        self.0.state = SessionState::oauth(provider, state);
    }
}

#[async_trait]
impl<T, S> FromRequestParts<S> for UnauthenticatedSession<T>
where
    T: HasSessionState + FromRequestParts<S> + Debug,
    <T as FromRequestParts<S>>::Rejection: Debug,
    S: Send + Sync,
    UnauthenticatedSession<T>: From<T>,
{
    type Rejection = InvalidSessionState;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let session = T::from_request_parts(parts, state).await.unwrap();

        match session.state() {
            SessionState::Unauthenticated => Ok(session.into()),
            session => {
                debug!("invalid session state, expected unauthenticated");
                Err(InvalidSessionState::from(session))
            }
        }
    }
}

impl From<Mutable> for UnauthenticatedSession<Mutable> {
    fn from(session: Mutable) -> Self {
        Self(session)
    }
}

impl From<Immutable> for UnauthenticatedSession<Immutable> {
    fn from(session: Immutable) -> Self {
        Self(session)
    }
}
