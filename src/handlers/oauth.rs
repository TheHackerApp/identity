use crate::state::{AllowedRedirectDomains, ApiUrl, AppState, FrontendUrl};
use axum::{
    extract::{Form, Path, Query, State},
    response::Redirect,
};
use database::{CustomDomain, Identity, PgPool, Provider, User};
use serde::Deserialize;
use session::extract::{
    CurrentUser, Mutable, OAuthSession, RegistrationNeededSession, UnauthenticatedSession,
};
use tracing::{error, info, instrument, Span};
use url::{Host, Url};

mod client;
mod error;

pub(crate) use client::Client;
use error::{Error, Result};

/// Start the OAuth2 login flow
#[instrument(
name = "oauth::launch", skip_all,
fields(
% slug,
return_to = params.return_to.as_ref().map(| u | u.as_str()).unwrap_or_default(),
)
)]
pub(crate) async fn launch(
    Path(slug): Path<String>,
    Query(params): Query<LaunchParams>,
    session: UnauthenticatedSession<Mutable>,
    State(url): State<ApiUrl>,
    State(client): State<Client>,
    State(db): State<PgPool>,
    State(allowed_redirect_domains): State<AllowedRedirectDomains>,
) -> Result<Redirect> {
    if let Some(return_to) = &params.return_to {
        if !redirect_url_is_valid(return_to, &db, allowed_redirect_domains).await? {
            return Err(Error::InvalidParameter("return-to"));
        }
    }

    if let Some(provider) = Provider::find_enabled(&slug, &db).await? {
        let redirect_url = url.join("/oauth/callback");
        let (url, state) = client.build_authorization_url(&provider.config, redirect_url.as_str());

        session.into_oauth(provider.slug, state, params.return_to);

        Ok(Redirect::to(&url))
    } else {
        Err(Error::UnknownProvider)
    }
}

/// Check if a redirect URL is valid without any additional context
async fn redirect_url_is_valid(
    url: &Url,
    db: &PgPool,
    allowed_redirect_domains: AllowedRedirectDomains,
) -> Result<bool> {
    // Require HTTPS-only URLs (but allows HTTP in development)
    #[cfg(debug_assertions)]
    let valid_scheme = url.scheme() == "http" || url.scheme() == "https";
    #[cfg(not(debug_assertions))]
    let valid_scheme = url.scheme() == "https";
    if !valid_scheme {
        return Ok(false);
    }

    // Only URLs with domains are allowed
    let Some(Host::Domain(domain)) = url.host() else {
        return Ok(false);
    };

    if allowed_redirect_domains.matches(domain) {
        Ok(true)
    } else {
        Ok(CustomDomain::exists(domain, db).await?)
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct LaunchParams {
    /// The URL to redirect the user back to
    return_to: Option<Url>,
}

/// Handle provider redirects and complete the login flow
#[instrument(
name = "oauth::callback",
skip_all,
fields(
state = % params.state,
success = matches ! (params.result, CallbackResult::Success { .. }),
provider.slug = session.provider,
provider.id,
return_to = session.return_to.as_ref().map(| u | u.as_str()).unwrap_or_default(),
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

            let url = session
                .return_to
                .as_ref()
                .map(|u| u.as_str())
                .unwrap_or_else(|| state.frontend_url.as_str())
                .to_owned(); // satisfying the borrow checker :(

            session.into_authenticated(identity.user_id);

            Ok(Redirect::to(&url))
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

#[instrument(name = "oauth::complete_registration", skip(state, session), fields(user.id = session.id))]
pub(crate) async fn complete_registration(
    State(state): State<AppState>,
    session: RegistrationNeededSession<Mutable>,
    Form(form): Form<RegistrationForm>,
) -> Result<Redirect> {
    let given_name = form.given_name.trim();
    if given_name.is_empty() {
        return Err(Error::InvalidParameter("givenName"));
    }
    let family_name = form.family_name.trim();
    if family_name.is_empty() {
        return Err(Error::InvalidParameter("familyName"));
    }

    let return_to = session
        .return_to
        .as_ref()
        .map(|u| u.as_str())
        .unwrap_or_else(|| state.frontend_url.as_str())
        .to_owned(); // satisfying the borrow checker :(

    let mut txn = state.db.begin().await?;

    let maybe_user = User::create(given_name, family_name, &session.email, &mut *txn).await;
    match maybe_user {
        Ok(user) => {
            Identity::link(
                &session.provider,
                user.id,
                &session.id,
                &session.email,
                &mut *txn,
            )
            .await?;

            session.into_authenticated(user.id);
        }
        Err(e) if e.is_unique_violation() => {}
        Err(e) => return Err(Error::Database(e)),
    }

    txn.commit().await?;

    Ok(Redirect::to(&return_to))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RegistrationForm {
    /// The user's given/first name
    given_name: String,
    /// The user's family/last name
    family_name: String,
}

#[instrument(name = "oauth::logout", skip_all, fields(user.id = session.id))]
pub(crate) async fn logout(
    session: CurrentUser<Mutable>,
    State(frontend_url): State<FrontendUrl>,
) -> Redirect {
    session.logout();

    Redirect::to(frontend_url.as_str())
}
