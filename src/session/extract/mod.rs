use super::SessionState;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Json, Response},
};
use serde::Serialize;

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
#[derive(Debug, Serialize)]
pub struct InvalidSessionState {
    #[serde(skip)]
    status: StatusCode,
    message: &'static str,
}

impl InvalidSessionState {
    /// Create a rejection from the app state and a session
    fn from(session: &SessionState) -> Self {
        let (status, message) = match session {
            SessionState::Unauthenticated | SessionState::OAuth(_) => {
                (StatusCode::UNAUTHORIZED, "unauthorized")
            }
            SessionState::RegistrationNeeded(_) => (StatusCode::FORBIDDEN, "registration required"),
            SessionState::Authenticated(_) => (StatusCode::FORBIDDEN, "forbidden"),
        };

        Self { status, message }
    }
}

impl IntoResponse for InvalidSessionState {
    fn into_response(self) -> Response {
        (self.status, Json(self)).into_response()
    }
}
