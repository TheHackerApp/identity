use axum::{routing::get, Router};
use database::PgPool;
use tower::ServiceBuilder;
use url::Url;

mod handlers;
mod logging;
mod oauth;
mod state;

pub(crate) use state::AppState;

/// Setup the routes
pub fn router(api_url: Url, db: PgPool, frontend_url: Url) -> Router {
    let state = AppState::new(api_url, db, frontend_url);

    Router::new()
        .route(
            "/graphql",
            get(handlers::playground).post(handlers::graphql),
        )
        .nest("/oauth", handlers::oauth())
        .with_state(state)
        .layer(ServiceBuilder::new().layer(logging::layer()))
}
