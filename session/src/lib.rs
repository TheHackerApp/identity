use axum_extra::extract::CookieJar;
use base64::prelude::{Engine, BASE64_URL_SAFE_NO_PAD};
use chrono::{DateTime, Duration, Utc};
use cookie::{Cookie, SameSite};
use hmac::{Hmac, Mac};
use rand::RngCore;
use redis::aio::ConnectionManager;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::sync::Arc;
use time::OffsetDateTime;
use tokio::sync::RwLock;
use tracing::{instrument, warn};
use url::Url;

mod error;
#[cfg(feature = "server")]
pub mod extract;
#[cfg(feature = "server")]
mod middleware;
mod store;

pub use error::Error;
use error::Result;
#[cfg(feature = "server")]
pub use middleware::SessionLayer;
use store::Store;

/// A shared reference to a session
pub type Handle = Arc<RwLock<Session>>;

const COOKIE_NAME: &str = "session";

/// length of the deserialized cookie in bytes
const COOKIE_SIZE: usize = 96;
/// length of the base64 url-encoded cookie
const SERIALIZED_LENGTH: usize = 128;
/// start position of the signature in the signed cookie
const SIGNATURE_START_INDEX: usize = 64;

#[cfg(feature = "server")]
/// Create a new session layer
pub fn layer(manager: Manager) -> SessionLayer {
    SessionLayer::new(manager)
}

/// A request session
#[derive(Debug, Deserialize, Serialize)]
pub struct Session {
    /// The unique ID to reference the session. Derived from a blake3 hash of the `cookie_value`.
    id: String,
    /// When the session expires
    expiry: DateTime<Utc>,
    pub state: SessionState,

    /// The value stored in the cookie
    #[serde(skip)]
    cookie_value: Option<Vec<u8>>,
}

impl Session {
    /// Generate a session ID from the value stored in the cookie
    pub(crate) fn generate_id(value: &[u8]) -> String {
        let hash = blake3::hash(value);
        BASE64_URL_SAFE_NO_PAD.encode(hash.as_bytes())
    }

    /// Get the session ID
    pub fn id(&self) -> &str {
        &self.id
    }

    /// If the session is expiring soon (within 8hrs), extend it another 3 days
    #[cfg(feature = "server")]
    pub(crate) fn extend_if_expiring(&mut self) {
        let now = Utc::now();
        if (self.expiry - Duration::hours(8)) < now {
            tracing::debug!("session about to expire, extending");
            self.expiry = now + Duration::days(3)
        }
    }
}

impl Default for Session {
    fn default() -> Self {
        let mut cookie_value = vec![0; 64];
        rand::thread_rng().fill_bytes(&mut cookie_value);

        Self {
            id: Self::generate_id(&cookie_value),
            expiry: Utc::now() + Duration::days(14),
            state: SessionState::default(),
            cookie_value: Some(cookie_value),
        }
    }
}

/// Manages user sessions
#[derive(Clone)]
pub struct Manager {
    store: Store,
    settings: Arc<CookieSettings>,
}

#[derive(Debug)]
pub(crate) struct CookieSettings {
    pub domain: String,
    pub key: String,
    pub secure: bool,
}

impl Manager {
    /// Create a new session manager
    pub fn new(cache: ConnectionManager, domain: &str, secure: bool, signing_key: &str) -> Self {
        let store = Store::new(cache);
        let settings = Arc::new(CookieSettings {
            domain: domain.to_owned(),
            secure,
            key: signing_key.to_owned(),
        });

        Self { store, settings }
    }

    /// Load the session from it's token
    #[instrument(name = "Manager::load_from_token", skip(self))]
    pub async fn load_from_token(&self, token: &str) -> Result<Option<Session>> {
        if token.len() != SERIALIZED_LENGTH {
            warn!(length = token.len(), "invalid session token length");
            return Ok(None);
        }

        let mut data = Vec::with_capacity(COOKIE_SIZE);
        if BASE64_URL_SAFE_NO_PAD.decode_vec(token, &mut data).is_err() {
            warn!("invalid base64 token");
            return Ok(None);
        }

        let (value, signature) = data.split_at(SIGNATURE_START_INDEX);

        let mut mac = Hmac::<Sha256>::new_from_slice(self.settings.key.as_bytes())
            .expect("key must be valid");
        mac.update(value);
        if mac.verify(signature.into()).is_err() {
            warn!("invalid HMAC");
            return Ok(None);
        }

        let id = Session::generate_id(value);
        self.store.load(&id).await
    }

    /// Load the session from cookies
    #[instrument(name = "Manager::load_from_cookie", skip_all)]
    pub async fn load_from_cookie(&self, jar: &CookieJar) -> Result<Option<Session>> {
        match jar.get(COOKIE_NAME) {
            Some(cookie) => self.load_from_token(cookie.value()).await,
            None => Ok(None),
        }
    }

    /// Save the session to the store
    #[instrument(name = "Manager::save", skip_all, fields(session.id = %session.id()))]
    pub async fn save(&self, session: &Session) -> Result<()> {
        self.store.save(session).await
    }

    /// Build a cookie from the session
    pub fn build_cookie(&self, session: Session) -> Option<Cookie<'static>> {
        let mut data = Vec::with_capacity(COOKIE_SIZE);
        data.extend_from_slice(&session.cookie_value?);

        let signature = {
            let mut mac = Hmac::<Sha256>::new_from_slice(self.settings.key.as_bytes())
                .expect("key must be valid");
            mac.update(&data);
            mac.finalize().into_bytes()
        };
        data.extend_from_slice(&signature);

        let (expiry, max_age) = {
            let nanos = session
                .expiry
                .timestamp_nanos_opt()
                .expect("timestamp must be valid") as i128;
            let expiry =
                OffsetDateTime::from_unix_timestamp_nanos(nanos).expect("timestamp must be valid");
            let max_age = expiry - OffsetDateTime::now_utc();
            (expiry, max_age)
        };

        Some(
            Cookie::build(COOKIE_NAME, BASE64_URL_SAFE_NO_PAD.encode(data))
                .http_only(true)
                .same_site(SameSite::Lax)
                .secure(self.settings.secure)
                .domain(self.settings.domain.clone())
                .expires(expiry)
                .max_age(max_age)
                .path("/")
                .finish(),
        )
    }
}

/// The authentication states a user can be in
#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", tag = "type")]
pub enum SessionState {
    /// User is not logged in (anonymous)
    #[default]
    Unauthenticated,
    /// Currently in OAuth flow (anonymous)
    #[serde(rename = "oauth")]
    OAuth(OAuthState),
    /// Needs to provide name (semi-anonymous)
    RegistrationNeeded(RegistrationNeededState),
    /// User is authenticated
    Authenticated(AuthenticatedState),
    // TODO: add state for impersonation
}

impl SessionState {
    /// Get the name of the state
    pub fn name(&self) -> &'static str {
        match self {
            Self::Unauthenticated => "unauthenticated",
            Self::OAuth(_) => "oauth",
            Self::RegistrationNeeded(_) => "registration needed",
            Self::Authenticated(_) => "authenticated",
        }
    }

    /// Get the ID of the user
    pub fn id(&self) -> Option<i32> {
        match self {
            Self::Authenticated(state) => Some(state.id),
            _ => None,
        }
    }

    /// Construct a new OAuth state
    #[cfg(feature = "server")]
    pub(crate) fn oauth(provider: String, state: String, return_to: Option<Url>) -> Self {
        Self::OAuth(OAuthState {
            provider,
            state,
            return_to,
        })
    }

    /// Construct a new registration needed state
    #[cfg(feature = "server")]
    pub(crate) fn registration_needed(id: String, email: String) -> Self {
        Self::RegistrationNeeded(RegistrationNeededState {
            id,
            email,
            return_to: None,
            provider: String::default(),
        })
    }

    /// Construct a new authenticated state
    #[cfg(feature = "server")]
    pub(crate) fn authenticated(id: i32) -> Self {
        Self::Authenticated(AuthenticatedState { id })
    }
}

/// Associated data for a user in the OAuth2 login flow
#[derive(Debug, Deserialize, Serialize)]
pub struct OAuthState {
    /// The slug of the provider we're authenticating with
    pub provider: String,
    /// Nonce used to prevent CSRF and clickjacking
    pub state: String,
    /// Where the user was redirected from
    pub return_to: Option<Url>,
}

/// Associated data for a user that needs to complete their registration
#[derive(Debug, Deserialize, Serialize)]
pub struct RegistrationNeededState {
    /// The slug of the provider the user authenticated with
    pub provider: String,
    /// The user's ID according to the provider
    pub id: String,
    /// The user's primary email
    pub email: String,
    /// Where the user was redirected from
    pub return_to: Option<Url>,
}

/// Associated data for an authenticated user
#[derive(Debug, Deserialize, Serialize)]
pub struct AuthenticatedState {
    /// The user's ID
    pub id: i32,
}
