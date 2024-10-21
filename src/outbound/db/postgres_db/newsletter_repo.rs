use async_trait::async_trait;

use super::*;
use crate::domain::new_subscriber::models::email::SubscriberEmail;
use crate::domain::new_subscriber::models::name::SubscriberName;
use crate::domain::newsletter::errors::NewsletterError;
use crate::domain::newsletter::models::confirmed_subscribers::ConfirmedSubscriber;
use futures::stream::{self, StreamExt};

impl PostgresDb {}

#[async_trait]
impl NewsletterRepository for PostgresDb {
    #[tracing::instrument(name = "Get confirmed subscribers", skip(self))]
    async fn get_confirmed_subscribers(
        &self,
    ) -> Result<Vec<Result<(ConfirmedSubscriber, SubscriptionToken), NewsletterError>>, anyhow::Error>
    {
        // Fetch confirmed subscribers from the database
        let confirmed_subscribers = sqlx::query!(
            r#"SELECT email, name, status, id FROM subscriptions WHERE status = $1"#,
            String::from(SubscriberStatus::SubscriptionConfirmed),
        )
        .fetch_all(&self.pool)
        .await?;

        let results: Vec<Result<(ConfirmedSubscriber, SubscriptionToken), NewsletterError>> =
            stream::iter(confirmed_subscribers)
                .then(|r| async move {
                    match SubscriberStatus::parse(&r.status) {
                        Ok(SubscriberStatus::SubscriptionConfirmed) => {
                            let name = SubscriberName::parse(r.name)?;
                            let email = SubscriberEmail::parse(r.email)?;
                            let confirmed_subscriber = NewSubscriber::build(name, email)
                                .with_id(Some(r.id))
                                .with_status(SubscriberStatus::SubscriptionConfirmed);
                            let subscriber_id = r.id;

                            let token = self.get_token_from_subscriber_id(subscriber_id).await?;

                            Ok((
                                ConfirmedSubscriber::new(confirmed_subscriber).unwrap(),
                                token,
                            ))
                        }
                        Err(error) => Err(NewsletterError::ValidationError(error.to_string())),
                        _ => Err(NewsletterError::NotFound(
                            "No confirmed subscribers found".to_string(),
                        )),
                    }
                })
                .collect()
                .await; // Collect the results

        Ok(results)
    }
}
