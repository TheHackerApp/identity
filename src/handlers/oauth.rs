use crate::state::ApiUrl;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json, Redirect, Response},
};
use database::{PgPool, Provider, ProviderConfiguration};
use rand::distributions::{Alphanumeric, DistString};
use serde::Serialize;
use tracing::{error, instrument};

/// Start the OAuth2 login flow
#[instrument(name = "oauth::launch", skip(db))]
pub(crate) async fn launch(
    Path(slug): Path<String>,
    State(url): State<ApiUrl>,
    State(db): State<PgPool>,
) -> Result<Redirect, Error> {
    if let Some(provider) = Provider::find_enabled(&slug, &db).await? {
        let redirect_url = url.join("/oauth/callback");
        let (url, _state) = build_authorization_url(&provider.config, redirect_url.as_str());

        // TODO: set state in session

        Ok(Redirect::to(&url))
    } else {
        Err(Error::NotFound)
    }
}

/// Build the OAuth2 authorize URL for the given service
fn build_authorization_url(config: &ProviderConfiguration, redirect_url: &str) -> (String, String) {
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

#[derive(Debug)]
pub(crate) enum Error {
    /// A database error
    Database(database::Error),
    /// The requested provider couldn't be found
    NotFound,
}

impl From<database::Error> for Error {
    fn from(error: database::Error) -> Self {
        Self::Database(error)
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        use std::error::Error;

        match self {
            Self::Database(error) => {
                match error.source() {
                    Some(source) => error!(%error, %source, "a database error occurred"),
                    None => error!(%error, "a database error occurred"),
                }
                response("internal error", StatusCode::INTERNAL_SERVER_ERROR)
            }
            Self::NotFound => response("unknown provider", StatusCode::NOT_FOUND),
        }
    }
}

/// A generic API error
#[derive(Serialize)]
struct ApiError<'m> {
    message: &'m str,
}

/// Generate an error response
#[inline(always)]
fn response<S: AsRef<str>>(message: S, code: StatusCode) -> Response {
    (
        code,
        Json(ApiError {
            message: message.as_ref(),
        }),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::build_authorization_url;
    use database::ProviderConfiguration;

    const ENCODED_REDIRECT_URI: &str = "https%3A%2F%2Fredirect.com%2Foauth%2Fcallback";

    #[test]
    fn build_authorize_url_google() {
        let config = ProviderConfiguration::Google {
            client_id: String::from("test-client-id"),
            client_secret: String::from("test-client-secret"),
        };

        let (url, state) = build_authorization_url(&config, "https://redirect.com/oauth/callback");
        assert_eq!(url, format!("https://accounts.google.com/o/oauth2/v2/auth?response_type=code&redirect_uri={ENCODED_REDIRECT_URI}&state={state}&client_id=test-client-id&scope=openid+profile+email"));
    }

    #[test]
    fn build_authorize_url_github() {
        let config = ProviderConfiguration::GitHub {
            client_id: String::from("test-client-id"),
            client_secret: String::from("test-client-secret"),
        };

        let (url, state) = build_authorization_url(&config, "https://redirect.com/oauth/callback");
        assert_eq!(url, format!("https://github.com/login/oauth/authorize?response_type=code&redirect_uri={ENCODED_REDIRECT_URI}&state={state}&client_id=test-client-id&scope=read%3Auser+user%3Aemail"));
    }

    #[test]
    fn build_authorize_url_discord() {
        let config = ProviderConfiguration::Discord {
            client_id: String::from("test-client-id"),
            client_secret: String::from("test-client-secret"),
        };

        let (url, state) = build_authorization_url(&config, "https://redirect.com/oauth/callback");
        assert_eq!(url, format!("https://discord.com/oauth2/authorize?response_type=code&redirect_uri={ENCODED_REDIRECT_URI}&state={state}&client_id=test-client-id&scope=identify+email"));
    }
}
