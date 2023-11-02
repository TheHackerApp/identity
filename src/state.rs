use crate::{oauth, session};
use axum::extract::FromRef;
use database::PgPool;
use std::sync::Arc;
use url::Url;

/// State passed to each request handler
#[derive(Clone)]
pub(crate) struct AppState {
    pub api_url: ApiUrl,
    pub db: PgPool,
    pub frontend_url: FrontendUrl,
    pub oauth_client: oauth::Client,
    pub schema: graphql::Schema,
    pub sessions: session::Manager,
}

impl AppState {
    pub fn new(
        api_url: Url,
        db: PgPool,
        frontend_url: Url,
        sessions: session::Manager,
    ) -> AppState {
        AppState {
            api_url: api_url.into(),
            db: db.clone(),
            frontend_url: frontend_url.into(),
            oauth_client: oauth::Client::default(),
            schema: graphql::schema(db),
            sessions,
        }
    }
}

impl FromRef<AppState> for ApiUrl {
    fn from_ref(state: &AppState) -> Self {
        state.api_url.clone()
    }
}

impl FromRef<AppState> for FrontendUrl {
    fn from_ref(state: &AppState) -> Self {
        state.frontend_url.clone()
    }
}

impl FromRef<AppState> for graphql::Schema {
    fn from_ref(state: &AppState) -> Self {
        state.schema.clone()
    }
}

impl FromRef<AppState> for oauth::Client {
    fn from_ref(state: &AppState) -> Self {
        state.oauth_client.clone()
    }
}

impl FromRef<AppState> for PgPool {
    fn from_ref(state: &AppState) -> Self {
        state.db.clone()
    }
}

impl FromRef<AppState> for session::Manager {
    fn from_ref(state: &AppState) -> Self {
        state.sessions.clone()
    }
}

/// The publicly accessible URL for the API
#[derive(Debug, Clone)]
pub(crate) struct ApiUrl(Arc<Url>);

impl ApiUrl {
    /// Append a path segment to the URL
    pub fn join(&self, path: &str) -> Url {
        self.0.join(path).expect("path must be valid")
    }
}

impl From<Url> for ApiUrl {
    fn from(url: Url) -> Self {
        Self(Arc::new(url))
    }
}

/// The publicly accessible URL for the accounts frontend
#[derive(Debug, Clone)]
pub(crate) struct FrontendUrl(Arc<Url>);

impl FrontendUrl {
    /// Convert the URL to a string slice
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// Append a path segment to the URL
    pub fn join(&self, path: &str) -> Url {
        self.0.join(path).expect("path must be valid")
    }
}

impl From<Url> for FrontendUrl {
    fn from(url: Url) -> Self {
        Self(Arc::new(url))
    }
}
