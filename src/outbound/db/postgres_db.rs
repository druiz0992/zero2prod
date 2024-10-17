use crate::configuration::DatabaseSettings;
use crate::domain::new_subscriber::errors::SubscriberError;
use crate::domain::new_subscriber::{
    models::{
        subscriber::{NewSubscriber, NewSubscriberRequest, SubscriberId, SubscriberStatus},
        token::SubscriptionToken,
    },
    ports::SubscriberRepository,
};
use crate::domain::newsletter::models::newsletter::Newsletter;
use crate::domain::newsletter::ports::NewsletterRepository;
use anyhow::Context;
use chrono::Utc;
use sqlx::postgres::PgPoolOptions;
use sqlx::{Executor, PgPool, Postgres, Transaction};

mod debug;
mod newsletter_repo;
mod subscriber_repo;

#[derive(Clone, Debug)]
pub struct PostgresDb {
    pool: PgPool,
}

impl PostgresDb {
    pub fn new(configuration: &DatabaseSettings) -> PostgresDb {
        PostgresDb {
            pool: PgPoolOptions::new()
                .acquire_timeout(std::time::Duration::from_secs(2))
                .connect_lazy_with(configuration.with_db()),
        }
    }

    #[tracing::instrument(
        name = "Checking if user is already subscribed",
        skip(self, subscriber)
    )]
    async fn get_subscriber(
        &self,
        subscriber: NewSubscriber,
    ) -> Result<NewSubscriber, SubscriberError> {
        let record = sqlx::query!(
            "SELECT id, email, name, status FROM subscriptions WHERE email = $1",
            subscriber.email.as_ref()
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| SubscriberError::Unexpected(anyhow::Error::from(e)))?;

        let (id, status) = match record {
            Some(existing_subscriber) if existing_subscriber.name == subscriber.name.as_ref() => {
                let parsed_status = SubscriberStatus::parse(&existing_subscriber.status)?;

                if existing_subscriber.name != subscriber.name.as_ref() {
                    return Err(SubscriberError::NotFound(format!(
                        "Subscriber with name {} and email {} not found",
                        subscriber.name.as_ref(),
                        subscriber.email.as_ref()
                    )));
                }
                (Some(existing_subscriber.id), parsed_status)
            }

            _ => (None, SubscriberStatus::NotInserted),
        };

        Ok(NewSubscriber::build(subscriber.name, subscriber.email)
            .with_id(id)
            .with_status(status))
    }

    #[tracing::instrument(
        name = "Saving new subscriber details in db",
        skip(new_subscriber, transaction)
    )]
    async fn insert_new(
        &self,
        transaction: &mut Transaction<'_, Postgres>,
        new_subscriber: NewSubscriber,
    ) -> Result<NewSubscriber, SubscriberError> {
        let subscriber_id = uuid::Uuid::new_v4();
        let query = sqlx::query!(
            r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at, status)
        VALUES ($1, $2, $3, $4, $5)
                "#,
            subscriber_id,
            new_subscriber.email.as_ref(),
            new_subscriber.name.as_ref(),
            Utc::now(),
            String::from(SubscriberStatus::SubscriptionPendingConfirmation)
        );
        transaction
            .execute(query)
            .await
            .map_err(|e| SubscriberError::Unexpected(anyhow::Error::from(e)))?;

        Ok(new_subscriber
            .with_id(Some(subscriber_id))
            .with_status(SubscriberStatus::SubscriptionPendingConfirmation))
    }

    #[tracing::instrument(name = "Get subscriber from subscriber_id", skip(self, id))]
    pub async fn get_subscriber_from_id(
        &self,
        id: uuid::Uuid,
    ) -> Result<NewSubscriber, SubscriberError> {
        let result = sqlx::query!(
            "SELECT  email, name, status FROM subscriptions WHERE id = $1",
            id
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| SubscriberError::Unexpected(anyhow::Error::from(e)))?;

        let subscriber_request = NewSubscriberRequest {
            email: result.email,
            name: result.name,
        };
        let subscriber: NewSubscriber = subscriber_request.try_into()?;

        let status = SubscriberStatus::parse(&result.status)?;

        Ok(subscriber.with_id(Some(id)).with_status(status))
    }
    #[tracing::instrument(name = "Get token from subscriber id", skip(self, subscriber_id))]
    async fn get_token_from_subscriber_id(
        &self,
        subscriber_id: uuid::Uuid,
    ) -> Result<SubscriptionToken, SubscriberError> {
        let result = sqlx::query!(
            r#"SELECT subscription_token FROM subscription_tokens WHERE  subscriber_id= $1"#,
            subscriber_id
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| SubscriberError::Unexpected(anyhow::Error::from(e)))?;

        SubscriptionToken::try_from(result.subscription_token).map_err(SubscriberError::from)
    }
}
