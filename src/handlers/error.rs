use axum::{
    http::{uri::InvalidUri, StatusCode},
    response::{IntoResponse, Json, Response},
};
use serde::Serialize;
use std::fmt::{Display, Formatter};
use tracing::error;

pub(crate) type Result<T, E = Error> = std::result::Result<T, E>;

/// Errors that can occur in request handlers
#[derive(Debug)]
pub(crate) enum Error {
    /// Could not find the specified event
    EventNotFound,
    Database(database::Error),
    Session(session::Error),
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EventNotFound => write!(f, "unknown event"),
            Self::Database(_) => write!(f, "unexpected database error"),
            Self::Session(_) => write!(f, "unexpected session error"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Database(e) => Some(e),
            Self::Session(e) => Some(e),
            Self::EventNotFound => None,
        }
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        use std::error::Error as _;

        match self {
            Self::EventNotFound => {
                return ApiError::response("unknown event", StatusCode::UNPROCESSABLE_ENTITY)
            }
            Self::Database(error) => match error.source() {
                Some(source) => error!(%error, %source, "unexpected database error"),
                None => error!(%error, "unexpected database error"),
            },
            Self::Session(error) => match error.source() {
                Some(source) => error!(%error, %source, "unexpected session error"),
                None => error!(%error, "unexpected session error"),
            },
        };

        ApiError::internal_server_error()
    }
}

impl From<database::Error> for Error {
    fn from(error: database::Error) -> Self {
        Self::Database(error)
    }
}

impl From<session::Error> for Error {
    fn from(error: session::Error) -> Self {
        Self::Session(error)
    }
}

impl From<InvalidUri> for Error {
    fn from(_: InvalidUri) -> Self {
        // if the domain is invalid, we know it can't be found
        Self::EventNotFound
    }
}

#[derive(Serialize)]
struct ApiError {
    message: &'static str,
}

impl ApiError {
    fn response(message: &'static str, status: StatusCode) -> Response {
        (status, Json(ApiError { message })).into_response()
    }

    fn internal_server_error() -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError {
                message: "internal server error",
            }),
        )
            .into_response()
    }
}
