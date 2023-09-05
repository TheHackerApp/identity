use axum::{routing::get, Router};
use database::PgPool;
use redis::aio::ConnectionManager as RedisConnectionManager;
use tower::ServiceBuilder;
use url::Url;

mod handlers;
mod logging;
mod oauth;
mod session;
mod state;

pub(crate) use state::AppState;

/// Setup the routes
pub fn router(
    api_url: Url,
    cache: RedisConnectionManager,
    db: PgPool,
    frontend_url: Url,
    cookie_signing_key: &str,
) -> Router {
    let sessions = session::Layer::new(
        cache,
        frontend_url.host_str().unwrap(),
        frontend_url.scheme() == "https",
        cookie_signing_key,
    );

    let state = AppState::new(api_url, db, frontend_url);

    Router::new()
        .route(
            "/graphql",
            get(handlers::playground).post(handlers::graphql),
        )
        .nest("/oauth", handlers::oauth())
        .with_state(state)
        .layer(
            ServiceBuilder::new()
                .layer(logging::layer())
                .layer(sessions),
        )
}
