use serde::{Deserialize, Serialize};
use sqlx::{
    database::{HasArguments, HasValueRef},
    encode::IsNull,
    error::BoxDynError,
    Database, Decode, Encode, Postgres,
};

/// A JSON type compatible with both [`async-graphql`] and [`sqlx`].
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Json<T>(pub T);

impl<T> From<T> for Json<T> {
    fn from(value: T) -> Self {
        Self(value)
    }
}

impl<T> From<sqlx::types::Json<T>> for Json<T> {
    fn from(value: sqlx::types::Json<T>) -> Self {
        Self(value.0)
    }
}

impl<T> AsRef<T> for Json<T> {
    fn as_ref(&self) -> &T {
        &self.0
    }
}

impl<T> std::ops::Deref for Json<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> std::borrow::Borrow<T> for Json<T> {
    fn borrow(&self) -> &T {
        &self.0
    }
}

impl<T, DB> sqlx::Type<DB> for Json<T>
where
    DB: Database,
    sqlx::types::Json<T>: sqlx::Type<DB>,
{
    fn type_info() -> DB::TypeInfo {
        <sqlx::types::Json<T> as sqlx::Type<DB>>::type_info()
    }

    fn compatible(ty: &DB::TypeInfo) -> bool {
        <sqlx::types::Json<T> as sqlx::Type<DB>>::compatible(ty)
    }
}

// Adapted from https://github.com/launchbadge/sqlx/blob/d0fbe7f/sqlx-postgres/src/types/json.rs#L57-L79
impl<'q, T> Encode<'q, Postgres> for Json<T>
where
    T: Serialize,
{
    fn encode_by_ref(&self, buf: &mut <Postgres as HasArguments<'q>>::ArgumentBuffer) -> IsNull {
        // JSONB version (as of 2020-03-20)
        buf.push(1);

        // the JSON data written to the buffer is the same regardless of parameter type
        serde_json::to_writer(&mut **buf, &self.0)
            .expect("failed to serialize to JSON for encoding on transmission to the database");

        IsNull::No
    }
}

// Adapted from https://github.com/launchbadge/sqlx/blob/d0fbe7f/sqlx-postgres/src/types/json.rs#L81-100
impl<'r, T: 'r> Decode<'r, Postgres> for Json<T>
where
    T: Deserialize<'r>,
{
    fn decode(value: <Postgres as HasValueRef<'r>>::ValueRef) -> Result<Self, BoxDynError> {
        let buf = value.as_bytes()?;

        // No need for check here since we don't use JSONb

        serde_json::from_slice(buf).map(Json).map_err(Into::into)
    }
}

#[cfg(feature = "graphql")]
#[async_graphql::Scalar(name = "JSON")]
impl<T> async_graphql::ScalarType for Json<T>
where
    T: serde::de::DeserializeOwned + Serialize + Send + Sync,
{
    fn parse(value: async_graphql::Value) -> async_graphql::InputValueResult<Self> {
        Ok(async_graphql::from_value(value)?)
    }

    fn to_value(&self) -> async_graphql::Value {
        async_graphql::to_value(&self.0).expect("JSON type must serialize")
    }
}
