use async_trait::async_trait;

use super::{
    models::{
        subscriber::{NewSubscriber, NewSubscriberRequest, SubscriberStatus},
        token::SubscriptionToken,
        token::SubscriptionTokenRequest,
    },
    ports::{
        SubscriberRepository, SubscriptionNotifier, SubscriptionService, SubscriptionServiceError,
    },
};

#[derive(Debug)]
pub struct Subscription<R, N>
where
    R: SubscriberRepository,
    N: SubscriptionNotifier,
{
    pub repo: R,
    pub notifier: N,
}

impl<R, N> Subscription<R, N>
where
    R: SubscriberRepository,
    N: SubscriptionNotifier,
{
    pub fn new(repo: R, notifier: N) -> Self {
        Self { repo, notifier }
    }
}

#[async_trait]
impl<R, N> SubscriptionService for Subscription<R, N>
where
    R: SubscriberRepository,
    N: SubscriptionNotifier,
{
    async fn new_subscriber(
        &self,
        subscriber_request: NewSubscriberRequest,
    ) -> Result<NewSubscriber, SubscriptionServiceError> {
        let subscription_token = SubscriptionToken::new();
        let (subscriber, token) = self
            .repo
            .retrieve_or_insert(subscriber_request, subscription_token)
            .await?;

        if subscriber.status == SubscriberStatus::SubscriptionPendingConfirmation {
            let message = self.notifier.build_notification(token)?;
            self.notifier
                .send_notification(&subscriber.email, &message)
                .await?
        }
        Ok(subscriber)
    }

    async fn confirm(
        &self,
        req: SubscriptionTokenRequest,
    ) -> Result<NewSubscriber, SubscriptionServiceError> {
        let subscription_token = SubscriptionTokenRequest::try_into(req)?;

        let subscriber = self.repo.retrieve_from_token(&subscription_token).await?;

        let subscriber = subscriber.with_status(SubscriberStatus::SubscriptionConfirmed);
        self.repo.update(subscriber.clone()).await?;
        Ok(subscriber)
    }
}
