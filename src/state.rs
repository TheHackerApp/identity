use crate::handlers::OAuthClient;
use axum::extract::FromRef;
use database::PgPool;
use globset::GlobSet;
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
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        api_url: Url,
        db: PgPool,
        frontend_url: Url,
        sessions: session::Manager,
        allowed_redirect_domains: GlobSet,
        domain_suffix: String,
        admin_domains: Vec<String>,
        user_domains: Vec<String>,
    ) -> AppState {
        let domains = Domains::new(domain_suffix, admin_domains, user_domains);
        AppState {
            allowed_redirect_domains: allowed_redirect_domains.into(),
            api_url: api_url.into(),
            db: db.clone(),
            domains: domains.clone(),
            frontend_url: frontend_url.into(),
            oauth_client: OAuthClient::default(),
            schema: graphql::schema(db, domains),
            sessions,
        }
    }
}
