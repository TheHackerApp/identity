use bytes::Bytes;
use redis::RedisError;
use std::fmt::{Display, Formatter};

pub(crate) type Result<T, E = Error> = std::result::Result<T, E>;

/// Errors that can occur when storing/retrieving sessions
#[derive(Debug)]
pub enum Error {
    /// Error while interacting with Redis
    Redis(RedisError),
    /// Unable to deserialize session
    Json {
        source: serde_json::Error,
        content: Bytes,
    },
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Redis(_) => write!(f, "error while interacting with redis"),
            Self::Json { content, .. } => {
                // content _should_ always be UTF8
                let content = String::from_utf8_lossy(content);
                write!(f, "failed to deserialize session: {content}")
            }
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Redis(e) => Some(e),
            Self::Json { source, .. } => Some(source),
        }
    }
}

impl From<RedisError> for Error {
    fn from(error: RedisError) -> Self {
        Self::Redis(error)
    }
}
