use async_trait::async_trait;

use super::{
    errors::SubscriberError,
    models::{
        subscriber::{NewSubscriber, NewSubscriberRequest, SubscriberStatus},
        token::SubscriptionToken,
        token::SubscriptionTokenRequest,
    },
    ports::{SubscriberRepository, SubscriptionNotifier, SubscriptionService},
};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct BlogSubscription<R, N>
where
    R: SubscriberRepository,
    N: SubscriptionNotifier,
{
    pub repo: Arc<R>,
    pub notifier: Arc<N>,
}

impl<R, N> BlogSubscription<R, N>
where
    R: SubscriberRepository,
    N: SubscriptionNotifier,
{
    pub fn new(repo: Arc<R>, notifier: Arc<N>) -> Self {
        Self { repo, notifier }
    }
}

#[async_trait]
impl<R, N> SubscriptionService for BlogSubscription<R, N>
where
    R: SubscriberRepository,
    N: SubscriptionNotifier,
{
    async fn new_subscriber(
        &self,
        subscriber_request: NewSubscriberRequest,
    ) -> Result<NewSubscriber, SubscriberError> {
        let subscription_token = SubscriptionToken::default();
        let (subscriber, token) = self
            .repo
            .retrieve_or_insert(subscriber_request, subscription_token)
            .await?;

        if subscriber.status == SubscriberStatus::SubscriptionPendingConfirmation {
            self.notifier
                .send_subscriber_notification(&subscriber.email, token)
                .await?
        }
        Ok(subscriber)
    }

    async fn confirm(
        &self,
        req: SubscriptionTokenRequest,
    ) -> Result<NewSubscriber, SubscriberError> {
        let subscription_token = SubscriptionTokenRequest::try_into(req)?;

        let mut subscriber = self.repo.retrieve_from_token(&subscription_token).await?;

        subscriber = subscriber.with_status(SubscriberStatus::SubscriptionConfirmed);
        self.repo.update(subscriber.clone()).await?;
        Ok(subscriber)
    }

    async fn delete(
        &self,
        req: SubscriptionTokenRequest,
    ) -> Result<NewSubscriber, SubscriberError> {
        let subscription_token = SubscriptionTokenRequest::try_into(req)?;

        let mut subscriber = self.repo.retrieve_from_token(&subscription_token).await?;

        if subscriber.status == SubscriberStatus::CancellationPendingConfirmation {
            self.repo.delete(subscriber.clone()).await?;
        } else {
            subscriber = subscriber.with_status(SubscriberStatus::CancellationPendingConfirmation);
            self.repo.update(subscriber.clone()).await?;
        }

        Ok(subscriber)
    }
}
