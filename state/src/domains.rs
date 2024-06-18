use globset::GlobSet;
use std::{collections::HashSet, sync::Arc};

/// Checks if the request domain is allowed to be redirected to
#[derive(Clone, Debug)]
pub struct AllowedRedirectDomains(Arc<GlobSet>);

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
pub struct Domains(Arc<DomainsInner>);

#[derive(Debug)]
struct DomainsInner {
    event_suffix: String,
    admin: HashSet<String>,
    user: HashSet<String>,
}

impl Domains {
    /// Create a new domain set
    pub fn new(
        event_suffix: String,
        admin_domains: Vec<String>,
        user_domains: Vec<String>,
    ) -> Self {
        let inner = DomainsInner {
            event_suffix,
            admin: admin_domains.into_iter().collect(),
            user: user_domains.into_iter().collect(),
        };
        Domains(Arc::new(inner))
    }

    /// Create an event domain from a slug
    pub fn for_event(&self, slug: &str) -> String {
        format!("{slug}{suffix}", suffix = &self.0.event_suffix)
    }

    /// Get the subdomain of a domain with respect to the current suffix
    pub fn extract_slug_for_subdomain<'a>(&'a self, domain: &'a str) -> Option<&str> {
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
