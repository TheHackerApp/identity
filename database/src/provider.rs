use crate::types::Json;
use serde::{Deserialize, Serialize};

/// Configuration for an authentication provider
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct Provider {
    /// A unique identifier for the provider
    pub slug: String,
    /// Whether the provider can be used for authentication
    pub enabled: bool,
    /// The display name
    pub name: String,
    /// The URL for the provider's icon
    pub icon: String,
    /// The client ID
    pub client_id: String,
    /// The client secret
    #[cfg_attr(feature = "graphql", graphql(skip))]
    pub client_secret: String,
    /// Provider-specific configuration, i.e. implementation kind, OIDC URLs, scopes, etc
    pub config: Json<ProviderConfiguration>,
}

/// The provider-specific configuration
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind")]
pub enum ProviderConfiguration {
    /// An OpenID connect provider
    OpenIDConnect {
        /// The URL to use for authorization according to RFC 6749 section 3.1
        authorization_endpoint: String,
        /// The URL to use for obtaining an access token according to RFC 6749 section 3.2
        token_endpoint: String,
        /// The URL to use for retrieving user info
        user_info_endpoint: String,
        /// The scopes to request when authorizing
        scopes: String,
    },
}
