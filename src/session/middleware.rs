use super::{store::Store, Handle, Session};
use axum::{
    http::{Request, StatusCode},
    response::{IntoResponse, Response},
};
use axum_extra::extract::CookieJar;
use futures::future::BoxFuture;
use redis::aio::ConnectionManager as RedisConnectionManager;
use std::{
    sync::Arc,
    task::{Context, Poll},
};
use tokio::sync::RwLock;
use tower::{Layer, Service};
use tracing::{error, info, instrument, Span};

/// Store and manage sessions
#[derive(Clone)]
pub struct SessionLayer {
    store: Store,
    settings: Arc<CookieSettings>,
}

#[derive(Debug)]
pub(crate) struct CookieSettings {
    pub domain: String,
    pub key: String,
    pub secure: bool,
}

impl SessionLayer {
    /// Create a new session layer
    pub fn new(
        cache: RedisConnectionManager,
        domain: &str,
        secure: bool,
        signing_key: &str,
    ) -> Self {
        let store = Store::new(cache);
        let settings = Arc::new(CookieSettings {
            domain: domain.to_owned(),
            secure,
            key: signing_key.to_owned(),
        });

        Self { store, settings }
    }

    /// Load the session by ID or initialize one
    #[instrument(name = "SessionLayer::load_or_create", skip(self))]
    async fn load_or_create(&self, id: Option<String>) -> Handle {
        if let Some(id) = id {
            let session = match self.store.load(&id).await {
                Ok(session) => session,
                Err(error) => {
                    use std::error::Error;
                    match error.source() {
                        Some(source) => error!(%error, %source, "failed to load source"),
                        None => error!(%error, "failed to load source"),
                    }
                    None
                }
            };

            Arc::new(RwLock::new(session.unwrap_or_default()))
        } else {
            Arc::new(RwLock::new(Session::default()))
        }
    }
}

impl<S> Layer<S> for SessionLayer {
    type Service = SessionMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        SessionMiddleware {
            inner,
            layer: self.clone(),
        }
    }
}

/// The middleware implementation
#[derive(Clone)]
pub struct SessionMiddleware<S> {
    inner: S,
    layer: SessionLayer,
}

impl<S, Body> Service<Request<Body>> for SessionMiddleware<S>
where
    S: Service<Request<Body>, Response = Response> + Clone + Send + 'static,
    Body: Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    #[instrument(name = "session", skip_all, fields(stage, user))]
    fn call(&mut self, mut req: Request<Body>) -> Self::Future {
        let layer = self.layer.clone();

        let inner = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, inner);

        Box::pin(async move {
            let jar = CookieJar::from_headers(req.headers());
            let session = Session::from_cookie(&jar, layer.settings.key.as_bytes());
            let session = layer.load_or_create(session).await;

            {
                let current = session.read().await;

                Span::current()
                    .record("stage", current.state.name())
                    .record("user", current.state.id());
                info!(id = %current.id, expires = %current.expiry, "loaded session");
            }

            req.extensions_mut().insert(session.clone());
            let response: S::Response = inner.call(req).await?;

            let mut session = Arc::try_unwrap(session)
                .expect("session still has owners")
                .into_inner();
            session.extend_if_expiring();

            if let Err(error) = layer.store.save(&session).await {
                use std::error::Error;

                match error.source() {
                    Some(source) => error!(%error, %source, "failed to save session"),
                    None => error!(%error, "failed to save session"),
                }

                return Ok((StatusCode::INTERNAL_SERVER_ERROR, "internal error").into_response());
            }

            if let Some(cookie) = session.into_cookie(&layer.settings) {
                let jar = jar.add(cookie);

                Ok((jar, response).into_response())
            } else {
                Ok(response)
            }
        })
    }
}
