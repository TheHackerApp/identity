use axum::{routing::get, Router};
use database::PgPool;
use tower::ServiceBuilder;

mod handlers;
mod logging;
mod state;

pub(crate) use state::AppState;

/// Setup the routes
pub fn router(db: PgPool) -> Router {
    let state = AppState::new(db);

    Router::new()
        .route(
            "/graphql",
            get(handlers::playground).post(handlers::graphql),
        )
        .with_state(state)
        .layer(ServiceBuilder::new().layer(logging::layer()))
}
