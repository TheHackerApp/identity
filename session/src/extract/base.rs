use crate::{Handle, Session, SessionState};
use axum::{async_trait, extract::FromRequestParts, http::request::Parts, Extension};
use tokio::sync::{OwnedRwLockReadGuard, OwnedRwLockWriteGuard};

/// Allow retrieval of the session state across mutable and immutable sessions
pub trait HasSessionState: private::Sealed {
    fn state(&self) -> &SessionState;
}

/// Extract an immutable session from the request extensions
#[derive(Debug)]
pub struct Immutable(pub(crate) OwnedRwLockReadGuard<Session>);

impl HasSessionState for Immutable {
    fn state(&self) -> &SessionState {
        &self.0.state
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for Immutable
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Extension(handle) = Extension::<Handle>::from_request_parts(parts, state)
            .await
            .expect("session extension missing, is the session::Layer installed?");
        let session = handle.read_owned().await;

        Ok(Self(session))
    }
}

impl AsRef<Session> for Immutable {
    fn as_ref(&self) -> &Session {
        &self.0
    }
}

impl std::borrow::Borrow<Session> for Immutable {
    fn borrow(&self) -> &Session {
        &self.0
    }
}

impl std::ops::Deref for Immutable {
    type Target = Session;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Extract a mutable session from the request extensions
#[derive(Debug)]
pub struct Mutable(pub(crate) OwnedRwLockWriteGuard<Session>);

impl HasSessionState for Mutable {
    fn state(&self) -> &SessionState {
        &self.0.state
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for Mutable
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Extension(handle) = Extension::<Handle>::from_request_parts(parts, state)
            .await
            .expect("session extension missing, is the session::Layer installed?");
        let session = handle.write_owned().await;

        Ok(Self(session))
    }
}

impl AsRef<Session> for Mutable {
    fn as_ref(&self) -> &Session {
        &self.0
    }
}

impl AsMut<Session> for Mutable {
    fn as_mut(&mut self) -> &mut Session {
        &mut self.0
    }
}

impl std::borrow::Borrow<Session> for Mutable {
    fn borrow(&self) -> &Session {
        &self.0
    }
}

impl std::borrow::BorrowMut<Session> for Mutable {
    fn borrow_mut(&mut self) -> &mut Session {
        &mut self.0
    }
}

impl std::ops::Deref for Mutable {
    type Target = Session;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for Mutable {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

mod private {
    pub trait Sealed {}

    impl Sealed for super::Immutable {}
    impl Sealed for super::Mutable {}
}
