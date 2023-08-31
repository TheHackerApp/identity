use crate::AppState;
use async_graphql::http::{playground_source, GraphQLPlaygroundConfig};
use async_graphql_axum::{GraphQLRequest, GraphQLResponse};
use axum::{extract::State, response::Html};
use tracing::instrument;

/// Handle graphql requests
#[instrument(name = "graphql", skip_all)]
pub(crate) async fn graphql(State(state): State<AppState>, req: GraphQLRequest) -> GraphQLResponse {
    let req = req.into_inner().data(state.db);
    state.schema.execute(req).await.into()
}

/// Serve the GraphQL playground for development
#[instrument(name = "playground")]
pub(crate) async fn playground() -> Html<String> {
    let config = GraphQLPlaygroundConfig::new("/graphql").title("Identity Playground");
    Html(playground_source(config))
}
