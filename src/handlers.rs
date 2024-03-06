use crate::AppState;
use ::context::{Scope, User};
use async_graphql::http::{playground_source, GraphQLPlaygroundConfig};
use async_graphql_axum::{GraphQLRequest, GraphQLResponse};
use axum::{
    extract::State,
    http::{
        header::{HeaderValue, CONTENT_TYPE},
        Method,
    },
    response::Html,
    routing::{get, post},
    Router,
};
use tower_http::cors::CorsLayer;
use tracing::instrument;
use url::Url;

mod context;
mod error;
mod oauth;

pub(crate) use context::context;
pub(crate) use oauth::Client as OAuthClient;

/// Create router for handling OAuth
pub(crate) fn oauth(frontend_url: &Url) -> Router<AppState> {
    let origin = HeaderValue::try_from(frontend_url.as_str().trim_end_matches('/')).unwrap();

    Router::new()
        .route("/launch/:provider", get(oauth::launch))
        .route("/callback", get(oauth::callback))
        .route(
            "/complete-registration",
            post(oauth::complete_registration).layer(
                CorsLayer::new()
                    .allow_methods(Method::POST)
                    .allow_headers([CONTENT_TYPE])
                    .allow_credentials(true)
                    .allow_origin(origin),
            ),
        )
        .route("/logout", get(oauth::logout))
}

/// Handle graphql requests
#[instrument(name = "graphql", skip_all)]
pub(crate) async fn graphql(
    State(schema): State<graphql::Schema>,
    scope: Scope,
    user: User,
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
