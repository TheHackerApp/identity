/// Configuration for an authentication provider
#[derive(Clone, Debug, Eq, PartialEq)]
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
    pub client_secret: String,
    /// Provider-specific configuration, i.e. implementation kind, OIDC URLs, scopes, etc
    pub config: String,
}
