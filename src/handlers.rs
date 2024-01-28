use crate::AppState;
use ::context::{scope, user};
use async_graphql::http::{playground_source, GraphQLPlaygroundConfig};
use async_graphql_axum::{GraphQLRequest, GraphQLResponse};
use axum::{
    extract::State,
    response::Html,
    routing::{get, post},
    Router,
};
use tracing::instrument;

mod context;
mod error;
mod oauth;

pub(crate) use context::context;
pub(crate) use oauth::Client as OAuthClient;

/// Create router for handling OAuth
pub(crate) fn oauth() -> Router<AppState> {
    Router::new()
        .route("/launch/:provider", get(oauth::launch))
        .route("/callback", get(oauth::callback))
        .route("/complete-registration", post(oauth::complete_registration))
        .route("/logout", get(oauth::logout))
}

/// Handle graphql requests
#[instrument(name = "graphql", skip_all)]
pub(crate) async fn graphql(
    State(schema): State<graphql::Schema>,
    scope: scope::Context,
    user: user::Context,
    req: GraphQLRequest,
) -> GraphQLResponse {
    let req = req.into_inner().data(scope).data(user);
    schema.execute(req).await.into()
}

/// Serve the GraphQL playground for development
#[instrument(name = "playground")]
pub(crate) async fn playground() -> Html<String> {
    let config = GraphQLPlaygroundConfig::new("/graphql").title("Identity Playground");
    Html(playground_source(config))
}
