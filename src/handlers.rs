use crate::{oauth, session, AppState};
use async_graphql::http::{playground_source, GraphQLPlaygroundConfig};
use async_graphql_axum::{GraphQLRequest, GraphQLResponse};
use axum::{
    extract::{Query, State},
    response::{Html, Json},
    routing::get,
    Router,
};
use database::{PgPool, User};
use serde::{Deserialize, Serialize};
use tracing::instrument;

mod error;

use crate::session::SessionState;
use error::Result;

/// Create router for handling OAuth
pub(crate) fn oauth() -> Router<AppState> {
    Router::new()
        .route("/launch/:provider", get(oauth::launch))
        .route("/callback", get(oauth::callback))
}

/// Handle graphql requests
#[instrument(name = "graphql", skip_all)]
pub(crate) async fn graphql(
    State(schema): State<graphql::Schema>,
    req: GraphQLRequest,
) -> GraphQLResponse {
    let req = req.into_inner();
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
    Query(params): Query<ContextParams>,
    State(db): State<PgPool>,
    State(sessions): State<session::Manager>,
) -> Result<Json<Option<UserInfo>>> {
    let Some(session) = sessions.load_from_token(&params.token).await? else {
        return Ok(Json(None));
    };

    let user_info = match session.state {
        SessionState::Unauthenticated => UserInfo::Unauthenticated,
        SessionState::OAuth(_) => UserInfo::OAuth,
        SessionState::RegistrationNeeded(state) => UserInfo::RegistrationNeeded {
            provider: state.provider,
            id: state.id,
            email: state.email,
        },
        SessionState::Authenticated(state) => {
            let user = User::find(state.id, &db).await?.expect("user must exist");

            // TODO: determine permissions

            UserInfo::Authenticated {
                id: user.id,
                given_name: user.given_name,
                family_name: user.family_name,
                email: user.primary_email,
                is_admin: user.is_admin,
            }
        }
    };

    Ok(Json(Some(user_info)))
}

/// The parameters for fetching the user context
#[derive(Deserialize)]
pub(crate) struct ContextParams {
    /// The session token
    token: String,
}

/// The generate user context
#[derive(Serialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub(crate) enum UserInfo {
    /// The user is currently unauthenticated
    Unauthenticated,
    /// The user is in the middle of logging in via OAuth
    #[serde(rename = "oauth")]
    OAuth,
    /// The user needs to complete their registration
    RegistrationNeeded {
        /// The slug of the provider the user authenticated with
        provider: String,
        /// The user's ID according to the provider
        id: String,
        /// The user's primary email
        email: String,
    },
    /// The user is fully authenticated
    Authenticated {
        /// The user's ID
        id: i32,
        /// The user's given/first name
        given_name: String,
        /// The user's family/last name
        family_name: String,
        /// The user's primary email
        email: String,
        /// Whether the user is an admin
        is_admin: bool,
    },
}
