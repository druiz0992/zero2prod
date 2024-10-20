use async_trait::async_trait;

use crate::domain::{
    auth::credentials::{Credentials, CredentialsError},
    newsletter::{
        errors::NewsletterError,
        models::newsletter::Newsletter,
        ports::{NewsletterNotifier, NewsletterRepository, NewsletterService},
    },
};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct BlogDelivery<R, N>
where
    R: NewsletterRepository,
    N: NewsletterNotifier,
{
    pub repo: Arc<R>,
    pub notifier: Arc<N>,
}

impl<R, N> BlogDelivery<R, N>
where
    R: NewsletterRepository,
    N: NewsletterNotifier,
{
    pub fn new(repo: Arc<R>, notifier: Arc<N>) -> Self {
        Self { repo, notifier }
    }
}

#[async_trait]
impl<R, N> NewsletterService for BlogDelivery<R, N>
where
    R: NewsletterRepository,
    N: NewsletterNotifier,
{
    async fn send_newsletter(
        &self,
        newsletter: Newsletter,
        base_url: &str,
    ) -> Result<(), NewsletterError> {
        let confirmed_subscribers_with_tokens = self.repo.get_confirmed_subscribers().await?;

        for subscriber_with_token in confirmed_subscribers_with_tokens {
            match subscriber_with_token {
                Ok(subscriber_with_token) => {
                    let subscriber = subscriber_with_token.0;
                    let token = subscriber_with_token.1;
                    tracing::Span::current().record(
                        "subscriber_email",
                        tracing::field::display(&subscriber.email().as_str()),
                    );
                    self.notifier
                        .send_newsletter(subscriber.email(), &newsletter, token, base_url)
                        .await?;
                }

                Err(error) => {
                    tracing::warn!(
                        error.cause_chain = ?error,
                        "Skipping a confirmed subscriber. Their stored contact details are invalid",
                    );
                }
            }
        }

        Ok(())
    }

    async fn validate_credentials(&self, credentials: Credentials) -> Result<(), CredentialsError> {
        tracing::Span::current()
            .record("username", tracing::field::display(credentials.username()));
        let stored_credentials = self
            .repo
            .get_stored_credentials(credentials.username())
            .await?;

        let user_id = credentials.validate(stored_credentials).await?;
        tracing::Span::current().record("user_id", tracing::field::display(&user_id));
        Ok(())
    }
}
