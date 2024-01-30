use super::client;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Json, Redirect, Response},
};
use serde::Serialize;
use tracing::error;
use url::Url;

pub(crate) type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug)]
pub(crate) enum Error {
    /// A database error
    Database(database::Error),
    /// The requested provider couldn't be found
    UnknownProvider,
    /// The provided state doesn't match the stored state
    InvalidState,
    /// An error response from the provider
    ProviderResponse(Url),
    /// An error occurred while interacting with the provider
    ProviderInteraction(client::Error),
    /// The value provided for the parameter was invalid
    InvalidParameter(&'static str),
}

impl From<database::SqlxError> for Error {
    fn from(error: database::SqlxError) -> Self {
        Self::Database(error.into())
    }
}

impl From<database::Error> for Error {
    fn from(error: database::Error) -> Self {
        Self::Database(error)
    }
}

impl From<client::Error> for Error {
    fn from(error: client::Error) -> Self {
        Self::ProviderInteraction(error)
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        use std::error::Error;

        match self {
            Self::Database(error) => {
                match error.source() {
                    Some(source) => error!(%error, %source, "a database error occurred"),
                    None => error!(%error, "a database error occurred"),
                }
                response("internal error", StatusCode::INTERNAL_SERVER_ERROR)
            }
            Self::UnknownProvider => response("unknown provider", StatusCode::NOT_FOUND),
            Self::InvalidState => response("invalid state", StatusCode::BAD_REQUEST),
            Self::ProviderResponse(url) => Redirect::to(url.as_str()).into_response(),
            Self::ProviderInteraction(error) => {
                match error.source() {
                    Some(source) => {
                        error!(%error, %source, "error while interacting with a provider")
                    }
                    None => error!(%error, "error while interacting with the provider"),
                }
                response("internal error", StatusCode::INTERNAL_SERVER_ERROR)
            }
            Self::InvalidParameter(param) => response(
                format!("invalid value for parameter {param:?}"),
                StatusCode::BAD_REQUEST,
            ),
        }
    }
}

/// A generic API error
#[derive(Serialize)]
struct ApiError<'m> {
    message: &'m str,
}

/// Generate an error response
#[inline(always)]
fn response<S: AsRef<str>>(message: S, code: StatusCode) -> Response {
    (
        code,
        Json(ApiError {
            message: message.as_ref(),
        }),
    )
        .into_response()
}
