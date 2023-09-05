use axum_extra::extract::CookieJar;
use base64::prelude::{Engine, BASE64_URL_SAFE_NO_PAD};
use chrono::{DateTime, Duration, Utc};
use cookie::{Cookie, SameSite};
use hmac::{Hmac, Mac};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::sync::Arc;
use time::OffsetDateTime;
use tokio::sync::RwLock;
use tracing::{debug, warn};

pub mod extract;
mod middleware;
mod store;

use middleware::CookieSettings;
pub use middleware::SessionLayer as Layer;

/// A shared reference to a session
pub type Handle = Arc<RwLock<Session>>;

const COOKIE_NAME: &str = "session";

/// length of the deserialized cookie in bytes
const COOKIE_SIZE: usize = 96;
/// length of the base64 url-encoded cookie
const SERIALIZED_LENGTH: usize = 128;
/// start position of the signature in the signed cookie
const SIGNATURE_START_INDEX: usize = 64;

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
    pub(crate) fn compute_id(value: &[u8]) -> String {
        let hash = blake3::hash(value);
        BASE64_URL_SAFE_NO_PAD.encode(hash.as_bytes())
    }

    /// Get the session ID
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Get the expiry of the session
    pub fn expiry(&self) -> DateTime<Utc> {
        self.expiry
    }

    /// Check if the session is expired
    pub fn is_expired(&self) -> bool {
        self.expiry < Utc::now()
    }

    /// If the session is expiring soon (within 8hrs), extend it another 3 days
    pub(crate) fn extend_if_expiring(&mut self) {
        let now = Utc::now();
        if (self.expiry - Duration::hours(8)) < now {
            debug!("session about to expire, extending");
            self.expiry = now + Duration::days(3)
        }
    }

    /// Build the session cookie if the cookie value is available
    pub(crate) fn into_cookie(self, settings: &CookieSettings) -> Option<Cookie<'static>> {
        let mut data = Vec::with_capacity(COOKIE_SIZE);
        data.extend_from_slice(&self.cookie_value?);

        let signature = {
            let mut mac =
                Hmac::<Sha256>::new_from_slice(settings.key.as_bytes()).expect("key must be valid");
            mac.update(&data);
            mac.finalize().into_bytes()
        };
        data.extend_from_slice(&signature);

        let (expiry, max_age) = {
            let nanos = self.expiry.timestamp_nanos() as i128;
            let expiry =
                OffsetDateTime::from_unix_timestamp_nanos(nanos).expect("timestamp must be valid");
            let max_age = expiry - OffsetDateTime::now_utc();
            (expiry, max_age)
        };

        Some(
            Cookie::build(COOKIE_NAME, BASE64_URL_SAFE_NO_PAD.encode(data))
                .http_only(true)
                .same_site(SameSite::Lax)
                .secure(settings.secure)
                .domain(settings.domain.clone())
                .expires(expiry)
                .max_age(max_age)
                .path("/")
                .finish(),
        )
    }

    /// Get the session's ID from the cookie, validating the signature
    pub(crate) fn from_cookie(jar: &CookieJar, signing_key: &[u8]) -> Option<String> {
        let cookie = jar.get(COOKIE_NAME)?;
        let signed_value = cookie.value();

        if signed_value.len() != SERIALIZED_LENGTH {
            warn!(length = signed_value.len(), "invalid session cookie length");
            return None;
        }

        let mut data = Vec::with_capacity(COOKIE_SIZE);
        BASE64_URL_SAFE_NO_PAD
            .decode_vec(signed_value, &mut data)
            .ok()?;

        let (value, signature) = data.split_at(SIGNATURE_START_INDEX);

        let mut mac = Hmac::<Sha256>::new_from_slice(signing_key).expect("key must be valid");
        mac.update(value);
        mac.verify(signature.into()).ok()?;

        Some(Self::compute_id(value))
    }
}

impl Default for Session {
    fn default() -> Self {
        let mut cookie_value = vec![0; 64];
        rand::thread_rng().fill_bytes(&mut cookie_value);

        Self {
            id: Self::compute_id(&cookie_value),
            expiry: Utc::now() + Duration::days(14),
            state: SessionState::default(),
            cookie_value: Some(cookie_value),
        }
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
    pub(crate) fn oauth(provider: String, state: String) -> Self {
        Self::OAuth(OAuthState { provider, state })
    }

    /// Construct a new registration needed state
    pub(crate) fn registration_needed(id: String, email: String) -> Self {
        Self::RegistrationNeeded(RegistrationNeededState { id, email })
    }

    /// Construct a new authenticated state
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
}

/// Associated data for a user that needs to complete their registration
#[derive(Debug, Deserialize, Serialize)]
pub struct RegistrationNeededState {
    /// The user's ID according to the provider
    pub id: String,
    /// The user's primary email
    pub email: String,
}

/// Associated data for an authenticated user
#[derive(Debug, Deserialize, Serialize)]
pub struct AuthenticatedState {
    /// The user's ID
    pub id: i32,
}
