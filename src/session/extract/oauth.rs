use super::{base::Mutable, InvalidSessionState, SessionState};
use crate::session::{OAuthState, Session};
use axum::{async_trait, extract::FromRequestParts, http::request::Parts};
use tokio::sync::OwnedRwLockWriteGuard;
use tracing::debug;

/// An in-progress OAuth session.
///
/// OAuth sessions can be converted to either a fully authenticated or registration session. Unless
/// explicitly converted to either one, it will automatically be converted to an unauthenticated
/// session upon leaving scope.
#[derive(Debug)]
pub struct OAuthSession(OwnedRwLockWriteGuard<Session>);

impl OAuthSession {
    /// Make the current session as authenticated
    pub fn into_authenticated(mut self, id: i32) {
        self.0.state = SessionState::authenticated(id);
    }

    /// Mark the current session as needing to complete registration
    pub fn into_registration_needed(mut self, id: String, email: String) {
        self.0.state = SessionState::registration_needed(id, email);
    }
}

impl std::ops::Deref for OAuthSession {
    type Target = OAuthState;

    fn deref(&self) -> &Self::Target {
        // We know this condition holds due to the FromRequestParts implementation
        match &self.0.state {
            SessionState::OAuth(state) => state,
            _ => unreachable!(),
        }
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for OAuthSession
where
    S: Send + Sync,
{
    type Rejection = InvalidSessionState;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let session = Mutable::from_request_parts(parts, state).await.unwrap();

        match &session.state {
            SessionState::OAuth(_) => Ok(OAuthSession(session.0)),
            session => {
                debug!("invalid session state, expected oauth");
                Err(InvalidSessionState::from(session))
            }
        }
    }
}

impl Drop for OAuthSession {
    fn drop(&mut self) {
        // If an OAuth session is not explicitly made successful, demote it to unauthenticated
        if matches!(&self.0.state, SessionState::OAuth(_)) {
            self.0.state = SessionState::Unauthenticated;
        }
    }
}
