use async_trait::async_trait;

use super::models::{
    subscriber::NewSubscriber, subscriber::NewSubscriberRequest, token::SubscriptionToken,
};
use crate::email_client::EmailClient;

#[async_trait]
pub trait SubscriptionRepository: Send + Sync + 'static {
    // if subscriber exists, returns subscriber + token. Else, inserts new subscriber
    async fn retrieve_or_insert(
        &self,
        subscriber: NewSubscriberRequest,
        token: SubscriptionToken,
    ) -> Result<(NewSubscriber, SubscriptionToken), anyhow::Error>;
}

#[async_trait]
pub trait SubscriptionService: Send + Sync + 'static {
    async fn new_subscriber(
        &self,
        req: NewSubscriberRequest,
        email_client: &EmailClient,
        base_url: &str,
    ) -> Result<NewSubscriber, anyhow::Error>;
}
