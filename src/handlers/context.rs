use super::error::{Error, Result};
use crate::state::Domains;
use axum::extract::{Query, State};
use context::{
    scope::{self, EventContext},
    user::{self, AuthenticatedContext, RegistrationNeededContext},
};
use database::{Event, PgPool, User};
use serde::Deserialize;
use session::SessionState;
use tracing::{info, instrument, Span};

#[derive(Deserialize)]
pub(crate) struct Params<'p> {
    #[serde(flatten)]
    scope: scope::Params<'p>,
    #[serde(flatten)]
    user: user::Params<'p>,
}

/// Determine the scope and user context for a request
#[instrument(name = "context", skip_all)]
pub(crate) async fn context(
    Query(params): Query<Params<'_>>,
    State(db): State<PgPool>,
    State(domains): State<Domains>,
    State(sessions): State<session::Manager>,
) -> Result<(scope::Context, user::Context)> {
    let scope = scope_context(params.scope, &db, domains).await?;
    let user = user_context(params.user, &db, sessions).await?;

    Ok((scope, user))
}

/// Determine the scope context for the request
#[instrument(name = "context::scope", skip_all, fields(domain, slug))]
async fn scope_context(
    params: scope::Params<'_>,
    db: &PgPool,
    domains: Domains,
) -> Result<scope::Context> {
    use scope::{Context, Params};

    let scope = match params {
        Params::Slug(slug) => {
            Span::current().record("slug", &*slug);
            let Some(event) = Event::find(&slug, db).await? else {
                return Err(Error::EventNotFound);
            };

            info!(scope = "event", %event.slug, %event.organization_id);

            Context::Event(EventContext {
                event: event.slug,
                organization_id: event.organization_id,
            })
        }
        Params::Domain(domain) => {
            Span::current().record("domain", &*domain);

            if domains.requires_admin(&domain) {
                info!(scope = "admin");
                Context::Admin
            } else if domains.requires_user(&domain) {
                info!(scope = "user");
                Context::User
            } else {
                let event = if let Some(slug) = domains.event_subdomain_for(&domain) {
                    info!(%slug, "handling hosted domain");
                    Event::find(slug, db).await?
                } else {
                    info!("handling custom domain");
                    Event::find_by_custom_domain(&domain, db).await?
                };
                let Some(event) = event else {
                    return Err(Error::EventNotFound);
                };

                info!(scope = "event", %event.slug, %event.organization_id);

                Context::Event(EventContext {
                    event: event.slug,
                    organization_id: event.organization_id,
                })
            }
        }
    };

    Ok(scope)
}

/// Get the user context for the request
#[instrument(name = "context::user", skip_all)]
async fn user_context(
    params: user::Params<'_>,
    db: &PgPool,
    sessions: session::Manager,
) -> Result<user::Context> {
    use user::Context;

    let session = sessions
        .load_from_token(&params.token)
        .await?
        .map(|s| s.state)
        .unwrap_or_default();

    let context = match session {
        SessionState::Unauthenticated => Context::Unauthenticated,
        SessionState::OAuth(_) => Context::OAuth,
        SessionState::RegistrationNeeded(state) => {
            Context::RegistrationNeeded(RegistrationNeededContext {
                provider: state.provider,
                id: state.id,
                email: state.email,
            })
        }
        SessionState::Authenticated(state) => {
            let user = User::find(state.id, db).await?.expect("user must exist");

            // TODO: determine permissions

            Context::Authenticated(AuthenticatedContext {
                id: user.id,
                given_name: user.given_name,
                family_name: user.family_name,
                email: user.primary_email,
                is_admin: user.is_admin,
            })
        }
    };

    Ok(context)
}
