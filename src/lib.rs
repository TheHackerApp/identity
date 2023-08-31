mod handlers;
mod logging;

use axum::{extract::FromRef, routing::get, Router};
use sqlx::PgPool;
use tower::ServiceBuilder;

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

#[derive(Clone)]
pub(crate) struct AppState {
    pub db: PgPool,
    pub schema: graphql::Schema,
}

impl AppState {
    pub fn new(db: PgPool) -> AppState {
        AppState {
            db,
            schema: graphql::schema(),
        }
    }
}

impl FromRef<AppState> for PgPool {
    fn from_ref(state: &AppState) -> Self {
        state.db.clone()
    }
}
