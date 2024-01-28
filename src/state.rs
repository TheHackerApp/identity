use crate::handlers::OAuthClient;
use axum::extract::FromRef;
use database::PgPool;
use globset::GlobSet;
use std::{collections::HashSet, sync::Arc};
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
        AppState {
            allowed_redirect_domains: allowed_redirect_domains.into(),
            api_url: api_url.into(),
            db: db.clone(),
            domains: Domains::new(domain_suffix, admin_domains, user_domains),
            frontend_url: frontend_url.into(),
            oauth_client: OAuthClient::default(),
            schema: graphql::schema(db),
            sessions,
        }
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

/// Checks if the request domain is allowed to be redirected to
#[derive(Clone, Debug)]
pub(crate) struct AllowedRedirectDomains(Arc<GlobSet>);

impl AllowedRedirectDomains {
    /// Test of a domain matches one that can be redirected to
    pub fn matches(&self, domain: &str) -> bool {
        self.0.is_match(domain)
    }
}

impl From<GlobSet> for AllowedRedirectDomains {
    fn from(matcher: GlobSet) -> Self {
        Self(Arc::new(matcher))
    }
}

/// A collection of domains to validate against
#[derive(Debug, Clone)]
pub(crate) struct Domains(Arc<DomainsInner>);

#[derive(Debug)]
struct DomainsInner {
    event_suffix: String,
    admin: HashSet<String>,
    user: HashSet<String>,
}

impl Domains {
    fn new(domain_suffix: String, admin_domains: Vec<String>, user_domains: Vec<String>) -> Self {
        let inner = DomainsInner {
            event_suffix: domain_suffix,
            admin: admin_domains.into_iter().collect(),
            user: user_domains.into_iter().collect(),
        };
        Domains(Arc::new(inner))
    }

    /// Get the subdomain of a domain with respect to the current suffix
    pub fn event_subdomain_for<'a>(&'a self, domain: &'a str) -> Option<&str> {
        domain.strip_suffix(&self.0.event_suffix)
    }

    /// Whether the domain requires admin permissions
    pub fn requires_admin(&self, domain: &str) -> bool {
        self.0.admin.contains(domain)
    }

    /// Whether the domain is scoped to a user
    pub fn requires_user(&self, domain: &str) -> bool {
        self.0.user.contains(domain)
    }
}
