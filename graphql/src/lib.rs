use async_graphql::{extensions::Analyzer, EmptySubscription, Schema as BaseSchema, SchemaBuilder};
use database::PgPool;

mod loaders;
mod mutation;
mod query;

use mutation::Mutation;
use query::Query;

/// The graphql schema for the service
pub type Schema = BaseSchema<Query, Mutation, EmptySubscription>;

/// Create a schema builder with the necessary extensions
fn builder() -> SchemaBuilder<Query, Mutation, EmptySubscription> {
    Schema::build(Query, Mutation::default(), EmptySubscription)
        .extension(logging::GraphQL)
        .extension(Analyzer)
}

/// Build the schema with the necessary extensions
pub fn schema(db: PgPool) -> Schema {
    builder()
        .data(loaders::identity_for_user(&db))
        .data(loaders::provider(&db))
        .data(loaders::user(&db))
        .data(loaders::user_by_primary_email(&db))
        .data(db)
        .finish()
}

/// Export the GraphQL schema
pub fn sdl() -> String {
    builder().finish().sdl()
}
