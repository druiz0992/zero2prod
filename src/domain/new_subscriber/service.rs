use async_trait::async_trait;

use super::{
    models::{
        subscriber::{self, NewSubscriber, NewSubscriberRequest, SubscriberStatus},
        token::SubscriptionToken,
    },
    ports::{SubscriptionRepository, SubscriptionService},
};
use crate::email_client::EmailClient;
use anyhow::Context;

#[derive(Debug)]
pub struct Subscription<R>
where
    R: SubscriptionRepository,
{
    pub repo: R,
}

impl<R> Subscription<R>
where
    R: SubscriptionRepository,
{
    pub fn new(repo: R) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl<R> SubscriptionService for Subscription<R>
where
    R: SubscriptionRepository,
{
    async fn new_subscriber(
        &self,
        subscriber_request: NewSubscriberRequest,
        email_client: &EmailClient,
        base_url: &str,
    ) -> Result<NewSubscriber, anyhow::Error> {
        let subscription_token = SubscriptionToken::new();
        let (subscriber, token) = self
            .repo
            .retrieve_or_insert(subscriber_request, subscription_token)
            .await?;

        if subscriber.status == SubscriberStatus::SubscriptionPendingConfirmation {
            send_confirmation_email(&email_client, &subscriber, &base_url, &token.as_ref())
                .await
                .context("Failed to send a confirmation email")?;
        }
        Ok(subscriber)
    }
}

#[tracing::instrument(
    name = "Send a confirmation email to a new subscriber",
    skip(email_client, new_subscriber, base_url)
)]
pub async fn send_confirmation_email(
    email_client: &EmailClient,
    new_subscriber: &NewSubscriber,
    base_url: &str,
    subscription_token: &str,
) -> Result<(), reqwest::Error> {
    let confirmation_link = format!(
        "{}/subscriptions/confirm?subscription_token={}",
        base_url, subscription_token
    );
    let plain_body = &format!(
        "Welcome to our newsletter!<br />\
            Click <a href=\"{}\">here</a> to confirm your subscription.",
        confirmation_link
    );
    let html_body = &format!(
        "Welcome to our newsletter!\nClick here {} to confirm your subscription.",
        confirmation_link
    );

    email_client
        .send_email(&new_subscriber.email, "Welcome", html_body, plain_body)
        .await
}
