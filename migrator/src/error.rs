use sqlx::migrate::MigrateError;
use std::{
    fmt::{self, Formatter},
    io,
};

#[derive(Debug)]
pub enum Error {
    Execute(sqlx::Error),
    Source(io::Error),
    VersionPreviouslyApplied(i64),
    VersionMismatch(i64),
    UnknownVersion(i64),
    VersionTooOld { target: i64, latest: i64 },
    VersionTooNew { target: i64, latest: i64 },
    Dirty(i64),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Execute(_) => write!(f, "error while executing migrations"),
            Self::Source(_) => write!(f, "error while interacting with source"),
            Self::VersionPreviouslyApplied(version) => write!(f, "migration {version} was previously applied, but is missing in the resolved migrations"),
            Self::VersionMismatch(version) => write!(f, "migration {version} was previously applied, but has been modified"),
            Self::UnknownVersion(version) => write!(f, "migration {version} does not exist in the source"),
            Self::VersionTooOld {target, latest} => write!(f, "migration {target} is older than the latest applied migration {latest}"),
            Self::VersionTooNew {target, latest} => write!(f, "migration {target} is newer than the latest applied migration {latest}"),
            Self::Dirty(version) => write!(f, "migration {version} is partially applied"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Execute(e) => Some(e),
            Self::Source(e) => Some(e),
            _ => None,
        }
    }
}

impl From<MigrateError> for Error {
    fn from(err: MigrateError) -> Self {
        match err {
            MigrateError::Execute(e) => Self::Execute(e),
            MigrateError::VersionMissing(v) => Self::VersionPreviouslyApplied(v),
            MigrateError::VersionMismatch(v) => Self::VersionMismatch(v),
            MigrateError::Dirty(v) => Self::Dirty(v),
            _ => unreachable!(),
        }
    }
}

impl From<sqlx::Error> for Error {
    fn from(err: sqlx::Error) -> Self {
        Self::Execute(err)
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Self::Source(err)
    }
}
