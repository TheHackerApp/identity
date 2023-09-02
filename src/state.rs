use axum::extract::FromRef;
use database::PgPool;

/// State passed to each request handler
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
