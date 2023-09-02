use database::ProviderConfiguration;
use rand::distributions::{Alphanumeric, DistString};
use reqwest::{
    header::{HeaderMap, HeaderValue, ACCEPT},
    Response, StatusCode,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{
    fmt::{Display, Formatter},
    time::Duration,
};
use tracing::instrument;

type Result<T, E = Error> = std::result::Result<T, E>;

/// The client for performing the different stages of the OAuth2 flow
#[derive(Clone)]
pub(crate) struct Client {
    client: reqwest::Client,
}

impl Client {
    /// Construct a new OAuth2 client
    pub fn new() -> Self {
        let headers = {
            let mut map = HeaderMap::new();
            map.insert(ACCEPT, HeaderValue::from_static("application/json"));
            map
        };

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .timeout(Duration::from_secs(5))
            .user_agent("the-hacker-app/identity")
            .build()
            .expect("client must build");

        Client { client }
    }

    /// Build the OAuth2 authorize URL for the given service
    pub fn build_authorization_url(
        &self,
        config: &ProviderConfiguration,
        redirect_url: &str,
    ) -> (String, String) {
        let state = Alphanumeric.sample_string(&mut rand::thread_rng(), 32);

        let mut params = form_urlencoded::Serializer::new(String::new());
        params.append_pair("response_type", "code");
        params.append_pair("redirect_uri", redirect_url);
        params.append_pair("state", &state);

        let url = match config {
            ProviderConfiguration::Google { client_id, .. } => {
                params.append_pair("client_id", client_id);
                params.append_pair("scope", "openid profile email");
                "https://accounts.google.com/o/oauth2/v2/auth"
            }
            ProviderConfiguration::GitHub { client_id, .. } => {
                params.append_pair("client_id", client_id);
                params.append_pair("scope", "read:user user:email");
                "https://github.com/login/oauth/authorize"
            }
            ProviderConfiguration::Discord { client_id, .. } => {
                params.append_pair("client_id", client_id);
                params.append_pair("scope", "identify email");
                "https://discord.com/oauth2/authorize"
            }
        };

        let params = params.finish();
        (format!("{url}?{params}"), state)
    }

    /// Perform the access token exchange, returning a bearer token
    #[instrument(name = "Client::exchange", skip_all, fields(kind = %provider.kind()))]
    pub async fn exchange(
        &self,
        code: &str,
        redirect_uri: &str,
        provider: &ProviderConfiguration,
    ) -> Result<String> {
        let config = ExchangeConfig::from(provider);
        let params = ExchangeRequest {
            code,
            grant_type: "authorization_code",
            client_id: config.client_id,
            client_secret: config.client_secret,
            redirect_uri,
        };
        let response = self.client.post(config.url).form(&params).send().await?;

        let creds = deserialize_if_successful::<ExchangeResponse>(response).await?;

        if creds.token_type.to_lowercase() == "bearer" {
            Ok(creds.access_token)
        } else {
            Err(Error::UnknownTokenType(creds.token_type))
        }
    }

    /// Retrieve information about the current user
    #[instrument(name = "Client::user_info", skip_all, fields(kind = %provider.kind()))]
    pub async fn user_info(
        &self,
        token: &str,
        provider: &ProviderConfiguration,
    ) -> Result<UserInfo> {
        match provider {
            ProviderConfiguration::Google { .. } => {
                self.simple_user_info::<OpenIDConnectUserInfo>(
                    "https://openidconnect.googleapis.com/v1/userinfo",
                    token,
                )
                .await
            }
            ProviderConfiguration::Discord { .. } => {
                self.simple_user_info::<DiscordUserInfo>(
                    "https://discord.com/api/v10/users/@me",
                    token,
                )
                .await
            }
            ProviderConfiguration::GitHub { .. } => {
                let (user_info, emails) = futures::try_join!(
                    self.github_request::<GitHubUserInfo>("https://api.github.com/user", token),
                    self.github_request::<Vec<GitHubEmail>>(
                        "https://api.github.com/user/emails",
                        token
                    )
                )?;

                let email = emails
                    .into_iter()
                    .find(|e| e.primary)
                    .map(|e| e.email)
                    .expect("user must have a primary email");

                Ok(UserInfo {
                    id: user_info.id.to_string(),
                    email,
                })
            }
        }
    }

    /// Fetch user info that simply requires data transformation
    #[instrument(name = "Client::simple_user_info", skip(self, token))]
    async fn simple_user_info<P>(&self, url: &str, token: &str) -> Result<UserInfo>
    where
        P: DeserializeOwned + Into<UserInfo>,
    {
        let response = self
            .client
            .get(url)
            .header("Authorization", format!("Bearer {token}"))
            .send()
            .await?;
        let provider_specific = deserialize_if_successful::<P>(response).await?;

        Ok(provider_specific.into())
    }

    /// Send an authenticated request to GitHub
    #[instrument(name = "Client::github_request", skip(self, token))]
    async fn github_request<R: DeserializeOwned>(&self, url: &str, token: &str) -> Result<R> {
        let response = self
            .client
            .get(url)
            .header("Authorization", format!("Bearer {token}"))
            .header("Accept", "application/vnd.github+json")
            .header("X-Github-Api-Version", "2022-11-28")
            .send()
            .await?;
        deserialize_if_successful(response).await
    }
}

impl Default for Client {
    fn default() -> Self {
        Client::new()
    }
}

/// Details about the authenticated user
#[derive(Debug)]
pub(crate) struct UserInfo {
    /// The user's ID according to the provider
    pub id: String,
    /// The user's preferred email
    pub email: String,
}

impl From<OpenIDConnectUserInfo> for UserInfo {
    fn from(user_info: OpenIDConnectUserInfo) -> Self {
        UserInfo {
            id: user_info.sub,
            email: user_info.email,
        }
    }
}

impl From<DiscordUserInfo> for UserInfo {
    fn from(user_info: DiscordUserInfo) -> Self {
        UserInfo {
            id: user_info.id,
            email: user_info.email,
        }
    }
}

/// An error from the OAuth client
#[derive(Debug)]
pub(crate) enum Error {
    /// The returned token is an unknown type
    UnknownTokenType(String),
    /// Invalid response body format
    BodyParse {
        source: serde_json::Error,
        content: String,
    },
    /// An unsuccessful response was received
    Unsuccessful { status: StatusCode, content: String },
    /// Failed to read response body
    BodyRead(reqwest::Error),
    /// Error while connecting to provider
    Connection(reqwest::Error),
    /// An unknown error occurred
    Unknown(reqwest::Error),
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::BodyRead(e) | Self::Connection(e) | Self::Unknown(e) => Some(e),
            Self::BodyParse { source, .. } => Some(source),
            _ => None,
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownTokenType(token) => write!(f, "unknown token type {token:?}"),
            Self::BodyParse { content, .. } => write!(f, "failed to parse body: {content:?}"),
            Self::Unsuccessful { status, content } => {
                write!(f, "unsuccessful response ({status}): {content:?}")
            }
            Self::BodyRead(_) => write!(f, "failed to read response body"),
            Self::Connection(_) => write!(f, "error while connecting to provider"),
            Self::Unknown(_) => write!(f, "an unknown error occurred"),
        }
    }
}

impl From<reqwest::Error> for Error {
    fn from(error: reqwest::Error) -> Self {
        if error.is_connect() || error.is_timeout() {
            Error::Connection(error)
        } else if error.is_body() || error.is_decode() {
            Error::BodyRead(error)
        } else {
            Error::Unknown(error)
        }
    }
}

/// User info from an OpenID Connect-compliant provider
#[derive(Debug, Deserialize)]
struct OpenIDConnectUserInfo {
    sub: String,
    email: String,
}

/// User info from Discord
#[derive(Debug, Deserialize)]
struct DiscordUserInfo {
    id: String,
    email: String,
}

/// User info from GitHub
#[derive(Debug, Deserialize)]
struct GitHubUserInfo {
    id: i64,
}

/// Entry in list of emails from GitHub
#[derive(Debug, Deserialize)]
struct GitHubEmail {
    email: String,
    primary: bool,
}

#[derive(Debug)]
struct ExchangeConfig<'e> {
    url: &'e str,
    client_id: &'e str,
    client_secret: &'e str,
}

impl<'e> From<&'e ProviderConfiguration> for ExchangeConfig<'e> {
    fn from(config: &'e ProviderConfiguration) -> Self {
        match config {
            ProviderConfiguration::Google {
                client_id,
                client_secret,
            } => ExchangeConfig {
                url: "https://oauth2.googleapis.com/token",
                client_id,
                client_secret,
            },
            ProviderConfiguration::GitHub {
                client_id,
                client_secret,
            } => ExchangeConfig {
                url: "https://github.com/login/oauth/access_token",
                client_id,
                client_secret,
            },
            ProviderConfiguration::Discord {
                client_id,
                client_secret,
            } => ExchangeConfig {
                url: "https://discord.com/api/oauth2/token",
                client_id,
                client_secret,
            },
        }
    }
}

#[derive(Debug, Serialize)]
struct ExchangeRequest<'e> {
    code: &'e str,
    grant_type: &'e str,
    client_id: &'e str,
    client_secret: &'e str,
    redirect_uri: &'e str,
}

#[derive(Debug, Deserialize)]
struct ExchangeResponse {
    access_token: String,
    token_type: String,
}

async fn deserialize_if_successful<T>(response: Response) -> Result<T, Error>
where
    T: DeserializeOwned,
{
    let status = response.status();
    let content = response.text().await?;

    if status.is_success() {
        serde_json::from_str(&content).map_err(|e| Error::BodyParse { source: e, content })
    } else {
        Err(Error::Unsuccessful { status, content })
    }
}

#[cfg(test)]
mod tests {
    use super::Client;
    use database::ProviderConfiguration;

    const ENCODED_REDIRECT_URI: &str = "https%3A%2F%2Fredirect.com%2Foauth%2Fcallback";

    #[test]
    fn build_authorize_url_google() {
        let config = ProviderConfiguration::Google {
            client_id: String::from("test-client-id"),
            client_secret: String::from("test-client-secret"),
        };

        let client = Client::default();
        let (url, state) =
            client.build_authorization_url(&config, "https://redirect.com/oauth/callback");
        assert_eq!(url, format!("https://accounts.google.com/o/oauth2/v2/auth?response_type=code&redirect_uri={ENCODED_REDIRECT_URI}&state={state}&client_id=test-client-id&scope=openid+profile+email"));
    }

    #[test]
    fn build_authorize_url_github() {
        let config = ProviderConfiguration::GitHub {
            client_id: String::from("test-client-id"),
            client_secret: String::from("test-client-secret"),
        };

        let client = Client::default();
        let (url, state) =
            client.build_authorization_url(&config, "https://redirect.com/oauth/callback");
        assert_eq!(url, format!("https://github.com/login/oauth/authorize?response_type=code&redirect_uri={ENCODED_REDIRECT_URI}&state={state}&client_id=test-client-id&scope=read%3Auser+user%3Aemail"));
    }

    #[test]
    fn build_authorize_url_discord() {
        let config = ProviderConfiguration::Discord {
            client_id: String::from("test-client-id"),
            client_secret: String::from("test-client-secret"),
        };

        let client = Client::default();
        let (url, state) =
            client.build_authorization_url(&config, "https://redirect.com/oauth/callback");
        assert_eq!(url, format!("https://discord.com/oauth2/authorize?response_type=code&redirect_uri={ENCODED_REDIRECT_URI}&state={state}&client_id=test-client-id&scope=identify+email"));
    }
}
