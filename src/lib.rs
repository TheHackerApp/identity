use axum::{routing::get, Router};
use database::PgPool;
use globset::GlobSet;
use redis::aio::ConnectionManager as RedisConnectionManager;
use url::Url;

mod handlers;
mod state;

pub(crate) use state::AppState;

/// Setup the routes
#[allow(clippy::too_many_arguments)]
pub fn router(
    api_url: Url,
    cache: RedisConnectionManager,
    db: PgPool,
    frontend_url: Url,
    allowed_redirect_domains: GlobSet,
    domain_suffix: String,
    admin_domains: Vec<String>,
    user_domains: Vec<String>,
    cookie_signing_key: &str,
) -> Router {
    let sessions = session::Manager::new(
        cache,
        frontend_url.host_str().unwrap(),
        frontend_url.scheme() == "https",
        cookie_signing_key,
    );

    let state = AppState::new(
        api_url,
        db,
        frontend_url,
        sessions.clone(),
        allowed_redirect_domains,
        domain_suffix,
        admin_domains,
        user_domains,
    );

    Router::new()
        .route("/context", get(handlers::context))
        .route(
            "/graphql",
            get(handlers::playground).post(handlers::graphql),
        )
        .nest("/oauth", handlers::oauth().layer(session::layer(sessions)))
        .with_state(state)
        .layer(logging::http())
}
