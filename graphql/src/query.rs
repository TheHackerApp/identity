use async_graphql::Object;

pub struct Query;

#[Object]
impl Query {
    /// Hello world
    async fn hello(&self) -> &'static str {
        "world"
    }
}
