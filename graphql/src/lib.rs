use async_graphql::{extensions::Analyzer, EmptySubscription, Schema as BaseSchema};

mod logging;
mod mutation;
mod query;

use mutation::Mutation;
use query::Query;

/// The graphql schema for the service
pub type Schema = BaseSchema<Query, Mutation, EmptySubscription>;

/// Build the schema with the necessary extensions
pub fn schema() -> Schema {
    Schema::build(Query, Mutation, EmptySubscription)
        .extension(logging::Logging)
        .extension(Analyzer)
        .finish()
}
