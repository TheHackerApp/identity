use async_graphql::{extensions::Analyzer, EmptyMutation, EmptySubscription, Schema as BaseSchema};

mod logging;
mod query;

use query::Query;

/// The graphql schema for the service
pub type Schema = BaseSchema<Query, EmptyMutation, EmptySubscription>;

/// Build the schema with the necessary extensions
pub fn schema() -> Schema {
    Schema::build(Query, EmptyMutation, EmptySubscription)
        .extension(logging::Logging)
        .extension(Analyzer)
        .finish()
}
