use crate::AppState;
use async_graphql::http::{playground_source, GraphQLPlaygroundConfig};
use async_graphql_axum::{GraphQLRequest, GraphQLResponse};
use axum::{
    extract::{Query, State},
    response::Html,
    routing::{get, post},
    Router,
};
use context::{
    scope,
    user::{self, AuthenticatedContext, RegistrationNeededContext},
};
use database::{PgPool, User};
use session::SessionState;
use tracing::instrument;

mod error;
mod oauth;

use error::Result;
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

/// Get the user context for the request
#[instrument(name = "context", skip_all)]
pub(crate) async fn context(
    Query(params): Query<user::Params<'_>>,
    State(db): State<PgPool>,
    State(sessions): State<session::Manager>,
) -> Result<user::Context> {
    use user::Context;

    let session = sessions
        .load_from_token(&params.token)
        .await?
        .map(|s| s.state)
        .unwrap_or_default();

    let context = match session {
        SessionState::Unauthenticated => Context::Unauthenticated,
        SessionState::OAuth(_) => Context::OAuth,
        SessionState::RegistrationNeeded(state) => {
            Context::RegistrationNeeded(RegistrationNeededContext {
                provider: state.provider,
                id: state.id,
                email: state.email,
            })
        }
        SessionState::Authenticated(state) => {
            let user = User::find(state.id, &db).await?.expect("user must exist");

            // TODO: determine permissions

            Context::Authenticated(AuthenticatedContext {
                id: user.id,
                given_name: user.given_name,
                family_name: user.family_name,
                email: user.primary_email,
                is_admin: user.is_admin,
            })
        }
    };

    Ok(context)
}
