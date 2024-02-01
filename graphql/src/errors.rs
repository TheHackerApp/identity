use async_graphql::{Error, ErrorExtensions};

/// An error raised when we do not know who the user is
#[derive(Debug)]
pub struct Unauthorized;

impl From<Unauthorized> for Error {
    fn from(_: Unauthorized) -> Self {
        Error::new("unauthorized")
            .extend_with(|_, extensions| extensions.set("code", "UNAUTHORIZED"))
    }
}

/// An error raised when the user has invalid permissions
#[derive(Debug)]
pub struct Forbidden;

impl From<Forbidden> for Error {
    fn from(_: Forbidden) -> Self {
        Error::new("forbidden").extend_with(|_, extensions| extensions.set("code", "FORBIDDEN"))
    }
}
