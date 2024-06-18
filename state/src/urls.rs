use std::sync::Arc;
use url::Url;

/// The publicly accessible URL for the API
#[derive(Debug, Clone)]
pub struct ApiUrl(Arc<Url>);

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
pub struct FrontendUrl(Arc<Url>);

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
