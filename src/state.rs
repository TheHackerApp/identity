use crate::handlers::OAuthClient;
use axum::extract::FromRef;
use database::PgPool;
use state::{AllowedRedirectDomains, ApiUrl, Domains, FrontendUrl};
use url::Url;

macro_rules! state {
    ( $( $field:ident : $type:ty ),+ $(,)? ) => {
        /// State passed to each request handler
        #[derive(Clone)]
        pub(crate) struct AppState {
            $( pub $field: $type, )*
        }

        $(
            impl FromRef<AppState> for $type {
                fn from_ref(state: &AppState) -> Self {
                    state.$field.clone()
                }
            }
        )*
    };
}

state! {
    allowed_redirect_domains: AllowedRedirectDomains,
    api_url: ApiUrl,
    db: PgPool,
    domains: Domains,
    frontend_url: FrontendUrl,
    oauth_client: OAuthClient,
    schema: graphql::Schema,
    sessions: session::Manager,
}

impl AppState {
    pub fn new(
        api_url: Url,
        db: PgPool,
        frontend_url: Url,
        portal_url: Url,
        sessions: session::Manager,
        allowed_redirect_domains: AllowedRedirectDomains,
        domains: Domains,
    ) -> AppState {
        AppState {
            allowed_redirect_domains,
            api_url: api_url.into(),
            db: db.clone(),
            domains: domains.clone(),
            frontend_url: frontend_url.into(),
            oauth_client: OAuthClient::default(),
            schema: graphql::schema(db, domains, portal_url),
            sessions,
        }
    }
}
