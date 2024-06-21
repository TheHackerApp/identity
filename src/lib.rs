use ::state::{AllowedRedirectDomains, Domains};
use axum::{routing::get, Router};
use database::PgPool;
use url::Url;

mod handlers;
mod state;

pub(crate) use state::AppState;

/// Setup the routes
pub fn router(
    api_url: Url,
    db: PgPool,
    frontend_url: Url,
    allowed_redirect_domains: AllowedRedirectDomains,
    domains: Domains,
    sessions: session::Manager,
) -> Router {
    let router = Router::new()
        .route("/context", get(handlers::context))
        .route(
            "/graphql",
            get(handlers::playground).post(handlers::graphql),
        )
        .nest(
            "/oauth",
            handlers::oauth(&frontend_url).layer(session::layer(sessions.clone())),
        )
        .with_state(AppState::new(
            api_url,
            db,
            frontend_url,
            sessions,
            allowed_redirect_domains,
            domains,
        ))
        .layer(logging::http());

    // Excludes the healthcheck from logging
    Router::new()
        .route("/health", get(handlers::health))
        .merge(router)
}
