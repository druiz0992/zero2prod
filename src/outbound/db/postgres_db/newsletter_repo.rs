use async_trait::async_trait;

use super::*;
use crate::domain::auth::credentials::StoredCredentials;
use crate::domain::new_subscriber::models::email::SubscriberEmail;
use crate::domain::new_subscriber::models::name::SubscriberName;
use crate::domain::newsletter::errors::NewsletterError;
use crate::domain::newsletter::models::confirmed_subscribers::ConfirmedSubscriber;
use futures::stream::{self, StreamExt};
use secrecy::Secret;

impl PostgresDb {}

#[async_trait]
impl NewsletterRepository for PostgresDb {
    #[tracing::instrument(name = "Get stored credentials", skip(username, self))]
    async fn get_stored_credentials(
        &self,
        username: &str,
    ) -> Result<Option<StoredCredentials>, NewsletterError> {
        let row = sqlx::query!(
            r#"SELECT user_id, password_hash FROM users WHERE username = $1"#,
            username,
        )
        .fetch_optional(&self.pool)
        .await
        .context("Failed to perform a query to retrieve stored credentials.")?
        .map(|row| StoredCredentials::new(row.user_id, row.password_hash));
        Ok(row)
    }

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
                        Ok(status) if status == SubscriberStatus::SubscriptionConfirmed => {
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
                        _ => Err(NewsletterError::NotFound(format!(
                            "No confirmed subscribers found"
                        ))),
                    }
                })
                .collect()
                .await; // Collect the results

        Ok(results)
    }
}
