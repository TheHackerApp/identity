use async_graphql::SimpleObject;

/// A reference to an organization on the `events` subgraph
#[derive(Debug, SimpleObject)]
#[graphql(unresolvable)]
pub struct Organization {
    pub id: i32,
}

/// A reference to an event on the `events` subgraph
#[derive(Debug, SimpleObject)]
#[graphql(unresolvable)]
pub struct Event {
    pub slug: String,
}
