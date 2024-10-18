use async_trait::async_trait;

use crate::domain::{
    auth::credentials::{Credentials, StoredCredentials},
    new_subscriber::models::{email::SubscriberEmail, token::SubscriptionToken},
    newsletter::{
        errors::NewsletterError,
        models::{confirmed_subscribers::ConfirmedSubscriber, newsletter::Newsletter},
    },
};

#[async_trait]
pub trait NewsletterRepository: Clone + Send + Sync + 'static {
    async fn get_stored_credentials(
        &self,
        username: &str,
    ) -> Result<Option<StoredCredentials>, NewsletterError>;
    async fn get_confirmed_subscribers(
        &self,
    ) -> Result<Vec<Result<(ConfirmedSubscriber, SubscriptionToken), NewsletterError>>, anyhow::Error>;
}

#[async_trait]
pub trait NewsletterService: Clone + Send + Sync + 'static {
    async fn send_newsletter(
        &self,
        credentials: Credentials,
        newsletter: Newsletter,
        base_url: &str,
    ) -> Result<(), NewsletterError>;
}

#[async_trait]
pub trait NewsletterNotifier: Clone + Send + Sync + 'static {
    async fn send_newsletter(
        &self,
        recipient: &SubscriberEmail,
        newsletter: &Newsletter,
        token: SubscriptionToken,
        base_url: &str,
    ) -> Result<(), NewsletterError>;
}
