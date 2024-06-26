use async_graphql::{
    extensions::Analyzer, EmptySubscription, SDLExportOptions, Schema as BaseSchema, SchemaBuilder,
};
use database::{loaders::RegisterDataLoaders, PgPool};
use state::Domains;
use url::Url;

mod entities;
mod errors;
mod mutation;
mod query;
mod webhooks;

use mutation::Mutation;
use query::Query;

/// The graphql schema for the service
pub type Schema = BaseSchema<Query, Mutation, EmptySubscription>;

/// Create a schema builder with the necessary extensions
fn builder() -> SchemaBuilder<Query, Mutation, EmptySubscription> {
    Schema::build(Query, Mutation::default(), EmptySubscription)
        .enable_federation()
        .extension(logging::GraphQL)
        .extension(Analyzer)
}

/// Build the schema with the necessary extensions
pub fn schema(db: PgPool, domains: Domains, portal_url: Url) -> Schema {
    let client = webhooks::Client::new(portal_url);

    builder()
        .register_dataloaders(&db)
        .data(client)
        .data(db)
        .data(domains)
        .finish()
}

/// Export the GraphQL schema
pub fn sdl() -> String {
    let options = SDLExportOptions::new()
        .federation()
        .include_specified_by()
        .compose_directive();
    builder().finish().sdl_with_options(options)
}
