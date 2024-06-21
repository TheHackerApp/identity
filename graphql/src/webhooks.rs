use reqwest::RequestBuilder;
use serde::Serialize;
use std::{sync::Arc, time::Duration};
use tracing::{error, instrument, span, Instrument, Level, Span};
use url::Url;

/// A webhook client for the portal service
#[derive(Clone)]
pub struct Client {
    client: reqwest::Client,
    url: Arc<Url>,
}

impl Client {
    pub fn new(url: Url) -> Self {
        let client = reqwest::Client::builder()
            .user_agent("the-hacker-app/identity")
            .timeout(Duration::from_secs(3))
            .build()
            .expect("client must build");

        Self {
            client,
            url: Arc::new(url),
        }
    }

    /// Notify of a participant's information changing
    #[instrument(name = "Client::on_participant_changed", skip(self))]
    pub fn on_participant_changed(&self, id: i32, email: &str) {
        let request = self
            .client
            .post(
                self.url
                    .join("/webhooks/participant")
                    .expect("url is always valid"),
            )
            .json(&Participant {
                id,
                primary_email: email,
            });

        self.dispatch("participant", request);
    }

    /// Dispatch an event in a background task
    fn dispatch(&self, kind: &'static str, request: RequestBuilder) {
        let span = span!(Level::INFO, "Client::dispatch", %kind);
        span.follows_from(Span::current());

        tokio::task::spawn(
            async move {
                let result = request
                    .send()
                    .await
                    .and_then(|response| response.error_for_status());

                if let Err(error) = result {
                    error!(%error, "failed to send webhook")
                }
            }
            .instrument(span),
        );
    }
}

#[derive(Serialize)]
struct Participant<'p> {
    id: i32,
    primary_email: &'p str,
}
