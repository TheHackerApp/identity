use super::SessionState;
use crate::state::FrontendUrl;
use axum::{
    extract::FromRef,
    response::{IntoResponse, Redirect, Response},
};
use url::Url;

mod base;
mod oauth;
mod registration_needed;
mod unauthenticated;
mod user;

pub use base::{Immutable, Mutable};
pub use oauth::OAuthSession;
pub use registration_needed::RegistrationNeededSession;
pub use unauthenticated::UnauthenticatedSession;
pub use user::CurrentUser;

/// A rejection generated when the requested session state did not match the
/// provided session state.
#[derive(Debug)]
pub struct InvalidSessionState(Url);

impl InvalidSessionState {
    /// Create a rejection from the app state and a session
    fn from<S>(state: &S, session: &SessionState) -> Self
    where
        FrontendUrl: FromRef<S>,
    {
        let path = match session {
            SessionState::Unauthenticated | SessionState::OAuth(_) => "/login",
            SessionState::RegistrationNeeded(_) => "/signup",
            SessionState::Authenticated(_) => "/",
        };

        let base = FrontendUrl::from_ref(state);
        Self(base.join(path))
    }
}

impl IntoResponse for InvalidSessionState {
    fn into_response(self) -> Response {
        Redirect::to(self.0.as_str()).into_response()
    }
}
