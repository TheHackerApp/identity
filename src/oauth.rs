use crate::{
    session::extract::{Mutable, OAuthSession, UnauthenticatedSession},
    state::{ApiUrl, AppState, FrontendUrl},
};
use axum::{
    extract::{Path, Query, State},
    response::Redirect,
};
use database::{Identity, PgPool, Provider};
use serde::Deserialize;
use tracing::{error, info, instrument, Span};

mod client;
mod error;

pub(crate) use client::Client;
use error::{Error, Result};

/// Start the OAuth2 login flow
#[instrument(name = "oauth::launch", skip_all, fields(%slug))]
pub(crate) async fn launch(
    Path(slug): Path<String>,
    session: UnauthenticatedSession<Mutable>,
    State(url): State<ApiUrl>,
    State(client): State<Client>,
    State(db): State<PgPool>,
) -> Result<Redirect> {
    if let Some(provider) = Provider::find_enabled(&slug, &db).await? {
        let redirect_url = url.join("/oauth/callback");
        let (url, state) = client.build_authorization_url(&provider.config, redirect_url.as_str());

        session.into_oauth(provider.slug, state);

        Ok(Redirect::to(&url))
    } else {
        Err(Error::UnknownProvider)
    }
}

/// Handle provider redirects and complete the login flow
#[instrument(
    name = "oauth::callback",
    skip_all,
    fields(
        state = %params.state,
        success = matches!(params.result, CallbackResult::Success { .. }),
        provider.slug = session.provider,
        provider.id,
    ),
)]
pub(crate) async fn callback(
    Query(params): Query<CallbackParams>,
    session: OAuthSession,
    State(state): State<AppState>,
) -> Result<Redirect> {
    if params.state != session.state {
        return Err(Error::InvalidState);
    }

    let code = params.result.into_code(&state.frontend_url)?;

    // Allow in-flight OAuth2 flows to finish even if it the provider was disabled
    let provider = Provider::find(&session.provider, &state.db)
        .await?
        .ok_or(Error::UnknownProvider)?;

    let token = state
        .oauth_client
        .exchange(
            &code,
            state.api_url.join("/oauth/callback").as_str(),
            &provider.config,
        )
        .await?;

    let user_info = state
        .oauth_client
        .user_info(&token, &provider.config)
        .await?;

    Span::current().record("provider.id", &user_info.id);
    info!("oauth2 flow complete");

    match Identity::find_by_remote_id(&session.provider, &user_info.id, &state.db).await? {
        Some(identity) => {
            info!(user.id = identity.user_id, "found existing user");
            session.into_authenticated(identity.user_id);

            // TODO: redirect to initial request or default page

            Ok(Redirect::to(state.frontend_url.as_str()))
        }
        None => {
            info!("user does not yet exist");
            session.into_registration_needed(user_info.id, user_info.email);

            Ok(Redirect::to(state.frontend_url.join("/signup").as_str()))
        }
    }
}

/// Params for an OAuth2 authorization code callback as defined by
/// [RFC6479 Section 4.1.2](https://datatracker.ietf.org/doc/html/rfc6749#section-4.1.2)
#[derive(Debug, Deserialize)]
pub(crate) struct CallbackParams {
    state: String,
    #[serde(flatten)]
    result: CallbackResult,
}

/// Differentiate between a successful and failure authorization code response
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub(crate) enum CallbackResult {
    Success {
        code: String,
    },
    Error {
        error: String,
        #[serde(rename = "error_description")]
        description: Option<String>,
        #[serde(rename = "error_uri")]
        uri: Option<String>,
    },
}

impl CallbackResult {
    /// Retrieve the authorization code or return with an error
    fn into_code(self, redirect: &FrontendUrl) -> Result<String> {
        match self {
            Self::Success { code } => Ok(code),
            Self::Error {
                error,
                description,
                uri,
            } => {
                let mut redirect = redirect.join("/login");

                let mut params = redirect.query_pairs_mut();
                if error == "access_denied" {
                    // This is a user error, display as such
                    params.append_pair("status", "cancelled");
                } else {
                    // These are _probably_ non-recoverable
                    error!(
                        code = %error,
                        description = %description.unwrap_or_default(),
                        uri = %uri.unwrap_or_default(),
                        "authorization request failed",
                    );

                    params.append_pair("status", "error");
                    params.append_pair("reason", "unknown");
                }
                drop(params);

                Err(Error::ProviderResponse(redirect))
            }
        }
    }
}
