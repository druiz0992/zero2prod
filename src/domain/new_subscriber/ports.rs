use async_trait::async_trait;

use super::errors::SubscriberError;
use super::models::{
    email::{EmailError, EmailMessage, SubscriberEmail},
    subscriber::{
        NewSubscriber, NewSubscriberRequest, SubscriberStatusError, SubscriberValidationError,
    },
    token::{SubscriptionToken, SubscriptionTokenError, SubscriptionTokenRequest},
};

#[async_trait]
///  Represents a store of subscriber data
pub trait SubscriberRepository: Send + Sync + 'static {
    /// Asynchronously retrieves a subscriber and token if it exists,
    ///  or creates a new entry for the provided `NewSubscriberRequest`
    async fn retrieve_or_insert(
        &self,
        subscriber: NewSubscriberRequest,
        token: SubscriptionToken,
    ) -> Result<(NewSubscriber, SubscriptionToken), SubscriberError>;

    /// Asynchronously updates a subscriber in repository
    async fn update(&self, subscriber: NewSubscriber) -> Result<(), SubscriberError>;

    /// Asynchronously retrieve a subscriber from a token
    async fn retrieve_from_token(
        &self,
        token: &SubscriptionToken,
    ) -> Result<NewSubscriber, SubscriberError>;

    async fn delete(&self, subscriber: NewSubscriber) -> Result<(), SubscriberError>;
}

#[async_trait]
pub trait SubscriptionService: Send + Sync + 'static {
    async fn new_subscriber(
        &self,
        req: NewSubscriberRequest,
    ) -> Result<NewSubscriber, SubscriberError>;

    async fn confirm(
        &self,
        req: SubscriptionTokenRequest,
    ) -> Result<NewSubscriber, SubscriberError>;

    async fn delete(&self, req: SubscriptionTokenRequest)
        -> Result<NewSubscriber, SubscriberError>;
}

#[async_trait]
pub trait SubscriptionNotifier: Send + Sync + 'static {
    async fn send_subscriber_notification(
        &self,
        recipient: &SubscriberEmail,
        token: SubscriptionToken,
    ) -> Result<(), SubscriberError>;
}
