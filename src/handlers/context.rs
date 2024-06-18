use super::error::{Error, Result};
use axum::{
    extract::{Query, State},
    http::uri::Authority,
};
use context::{
    AuthenticatedUser, EventScope, Scope, ScopeParams, User as UserContext, UserParams,
    UserRegistrationNeeded, UserRole,
};
use database::{Event, PgPool, User};
use serde::Deserialize;
use session::SessionState;
use state::Domains;
use tracing::{info, instrument, Span};

#[derive(Deserialize)]
pub(crate) struct Params<'p> {
    #[serde(flatten)]
    scope: ScopeParams<'p>,
    #[serde(flatten)]
    user: UserParams<'p>,
}

/// Determine the scope and user context for a request
#[instrument(name = "context", skip_all)]
pub(crate) async fn context(
    Query(params): Query<Params<'_>>,
    State(db): State<PgPool>,
    State(domains): State<Domains>,
    State(sessions): State<session::Manager>,
) -> Result<(Scope, UserContext)> {
    let scope = determine_scope_context(params.scope, &db, domains).await?;
    let user = determine_user_context(params.user, &db, &scope, sessions).await?;

    Ok((scope, user))
}

/// Determine the scope context for the request
#[instrument(name = "scope", skip_all, fields(domain, slug))]
async fn determine_scope_context(
    params: ScopeParams<'_>,
    db: &PgPool,
    domains: Domains,
) -> Result<Scope> {
    let scope = match params {
        ScopeParams::Slug(slug) => {
            Span::current().record("slug", &*slug);
            let Some(event) = Event::find(&slug, db).await? else {
                return Err(Error::EventNotFound);
            };

            info!(scope = "event", %event.slug, %event.organization_id);

            Scope::Event(EventScope {
                event: event.slug,
                organization_id: event.organization_id,
            })
        }
        ScopeParams::Domain(domain) => {
            let authority = Authority::try_from(&*domain)?;
            let host = authority.host();

            Span::current().record("domain", host);

            if domains.requires_admin(host) {
                info!(scope = "admin");
                Scope::Admin
            } else if domains.requires_user(host) {
                info!(scope = "user");
                Scope::User
            } else {
                let event = if let Some(slug) = domains.extract_slug_for_subdomain(host) {
                    info!(%slug, "handling hosted domain");
                    Event::find(slug, db).await?
                } else {
                    info!("handling custom domain");
                    Event::find_by_custom_domain(host, db).await?
                };
                let Some(event) = event else {
                    return Err(Error::EventNotFound);
                };

                info!(scope = "event", %event.slug, %event.organization_id);

                Scope::Event(EventScope {
                    event: event.slug,
                    organization_id: event.organization_id,
                })
            }
        }
    };

    Ok(scope)
}

/// Get the user context for the request
#[instrument(name = "user", skip_all)]
async fn determine_user_context(
    params: UserParams<'_>,
    db: &PgPool,
    scope: &Scope,
    sessions: session::Manager,
) -> Result<UserContext> {
    let session = sessions
        .load_from_token(&params.token)
        .await?
        .map(|s| s.state)
        .unwrap_or_default();

    let context = match session {
        SessionState::Unauthenticated => UserContext::Unauthenticated,
        SessionState::OAuth(_) => UserContext::OAuth,
        SessionState::RegistrationNeeded(state) => {
            UserContext::RegistrationNeeded(UserRegistrationNeeded {
                provider: state.provider,
                id: state.id,
                email: state.email,
            })
        }
        SessionState::Authenticated(state) => {
            // TODO: handle user not existing
            let user = User::find(state.id, db).await?.expect("user must exist");
            let role = determine_role(scope, &user, db).await?;

            UserContext::Authenticated(AuthenticatedUser {
                id: user.id,
                given_name: user.given_name,
                family_name: user.family_name,
                email: user.primary_email,
                role,
                is_admin: user.is_admin,
            })
        }
    };

    Ok(context)
}

/// Determine the role for the current user
#[instrument(skip_all, fields(%user.id, role))]
async fn determine_role(scope: &Scope, user: &User, db: &PgPool) -> Result<Option<UserRole>> {
    let Scope::Event(event) = scope else {
        return Ok(None);
    };

    // Being a participant takes precedence over being an organizer as it is more granular
    if User::is_participant(user.id, &event.event, db).await? {
        Span::current().record("role", "participant");
        return Ok(Some(UserRole::Participant));
    }

    if let Some(role) = User::is_organizer(user.id, event.organization_id, db).await? {
        Span::current().record("role", tracing::field::debug(role));
        return Ok(Some(role.into()));
    }

    Ok(None)
}
