use crate::configuration::DatabaseSettings;
use crate::domain::new_subscriber::{
    models::{
        subscriber::{NewSubscriber, NewSubscriberRequest, SubscriberId, SubscriberStatus},
        token::SubscriptionToken,
    },
    ports::{SubscriberRepository, SubscriberRepositoryError},
};
use anyhow::Context;
use async_trait::async_trait;
use chrono::Utc;
use sqlx::postgres::PgPoolOptions;
use sqlx::{Executor, PgPool, Postgres, Transaction};

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

    // TODO: This is only for testing
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    #[tracing::instrument(
        name = "Checking if user is already subscribed",
        skip(self, subscriber)
    )]
    async fn get_subscriber(
        &self,
        subscriber: NewSubscriber,
    ) -> Result<NewSubscriber, SubscriberRepositoryError> {
        let record = sqlx::query!(
            "SELECT id, email, name, status FROM subscriptions WHERE email = $1",
            subscriber.email.as_ref()
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| SubscriberRepositoryError::Unexpected(anyhow::Error::from(e)))?;

        let (id, status) = match record {
            Some(existing_subscriber) if existing_subscriber.name == subscriber.name.as_ref() => {
                let parsed_status = SubscriberStatus::parse(&existing_subscriber.status)?;

                if existing_subscriber.name != subscriber.name.as_ref() {
                    return Err(SubscriberRepositoryError::SubscriberNotFound);
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
    ) -> Result<NewSubscriber, SubscriberRepositoryError> {
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
            .map_err(|e| SubscriberRepositoryError::Unexpected(anyhow::Error::from(e)))?;

        Ok(new_subscriber
            .with_id(Some(subscriber_id))
            .with_status(SubscriberStatus::SubscriptionPendingConfirmation))
    }

    #[tracing::instrument(
        name = "Store subscription token in the database",
        skip(subscription_token, transaction)
    )]
    async fn store_token(
        &self,
        transaction: &mut Transaction<'_, Postgres>,
        subscription_token: &SubscriptionToken,
        subscriber_id: SubscriberId,
    ) -> Result<(), sqlx::Error> {
        let query = sqlx::query!(
            r#"INSERT INTO subscription_tokens (subscription_token, subscriber_id)
            VALUES ($1, $2)"#,
            subscription_token.as_ref(),
            subscriber_id.unwrap(),
        );
        transaction.execute(query).await?;
        Ok(())
    }

    #[tracing::instrument(name = "Get token from subscriber id", skip(self, subscriber_id))]
    async fn get_token_from_subscriber_id(
        &self,
        subscriber_id: uuid::Uuid,
    ) -> Result<SubscriptionToken, SubscriberRepositoryError> {
        let result = sqlx::query!(
            r#"SELECT subscription_token FROM subscription_tokens WHERE  subscriber_id= $1"#,
            subscriber_id
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| SubscriberRepositoryError::Unexpected(anyhow::Error::from(e)))?;

        SubscriptionToken::try_from(result.subscription_token)
            .map_err(SubscriberRepositoryError::from)
    }

    #[tracing::instrument(name = "Get subscriber_id from token", skip(self, subscription_token))]
    pub async fn get_subscriber_id_from_token(
        &self,
        subscription_token: &SubscriptionToken,
    ) -> Result<SubscriberId, sqlx::Error> {
        let result = sqlx::query!(
            r#"SELECT subscriber_id FROM subscription_tokens WHERE subscription_token = $1"#,
            subscription_token.as_ref(),
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.map(|r| r.subscriber_id))
    }

    #[tracing::instrument(name = "Get subscriber from subscriber_id", skip(self, id))]
    pub async fn get_subscriber_from_id(
        &self,
        id: uuid::Uuid,
    ) -> Result<NewSubscriber, SubscriberRepositoryError> {
        let result = sqlx::query!(
            "SELECT  email, name, status FROM subscriptions WHERE id = $1",
            id
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| SubscriberRepositoryError::Unexpected(anyhow::Error::from(e)))?;

        let subscriber_request = NewSubscriberRequest {
            email: result.email,
            name: result.name,
        };
        let subscriber: NewSubscriber = subscriber_request.try_into()?;

        let status = SubscriberStatus::parse(&result.status)?;

        Ok(subscriber.with_id(Some(id)).with_status(status))
    }

    #[tracing::instrument(name = "Mark subscriber as confirmed", skip(subscriber_id, self))]
    pub async fn confirm_subscriber(&self, subscriber_id: uuid::Uuid) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"UPDATE subscriptions SET status = $1 WHERE id = $2"#,
            String::from(SubscriberStatus::SubscriptionConfirmed),
            subscriber_id,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

#[async_trait]
impl SubscriberRepository for PostgresDb {
    #[tracing::instrument(
        name = "Adding a new subscriber",
        skip(self, subscriber_request, token)
    )]
    async fn retrieve_or_insert(
        &self,
        subscriber_request: NewSubscriberRequest,
        token: SubscriptionToken,
    ) -> Result<(NewSubscriber, SubscriptionToken), SubscriberRepositoryError> {
        let mut new_token = token;
        let mut new_subscriber: NewSubscriber = subscriber_request.try_into()?;

        new_subscriber = self
            .get_subscriber(new_subscriber)
            .await
            .context("Failed to check if subscriber existed in db")?;

        if let Some(subscriber_id) = new_subscriber.id {
            new_token = self
                .get_token_from_subscriber_id(subscriber_id)
                .await
                .context("Failed to read token from database")?;
        } else {
            let mut transaction = self
                .pool
                .begin()
                .await
                .context("Failed to acquire a Postgress connection from the pool")?;

            new_subscriber = self
                .insert_new(&mut transaction, new_subscriber)
                .await
                .context("Failed to insert a new subscriber in the database")?;

            self.store_token(&mut transaction, &new_token, new_subscriber.id)
                .await
                .context("Failed to store the confirmation token for a new subscriber.")?;

            transaction
                .commit()
                .await
                .context("Failed to commit SQL transaction to store a new subscriber")?;
        }
        Ok((new_subscriber, new_token))
    }

    #[tracing::instrument(name = "Update subscriber", skip(subscriber, self))]
    async fn update(&self, subscriber: NewSubscriber) -> Result<(), SubscriberRepositoryError> {
        let result = sqlx::query!(
            r#"UPDATE subscriptions SET email = $1, name = $2, status = $3 WHERE id = $4"#,
            subscriber.email.as_ref(),
            subscriber.name.as_ref(),
            String::from(subscriber.status),
            subscriber.id,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| SubscriberRepositoryError::Unexpected(anyhow::Error::from(e)))
        .context("Failed to update subscriber to database")?;

        if result.rows_affected() == 0 {
            return Err(SubscriberRepositoryError::SubscriberNotFound);
        }

        Ok(())
    }

    #[tracing::instrument(name = "Retrieve subscriber from token", skip(token, self))]
    async fn retrieve_from_token(
        &self,
        token: &SubscriptionToken,
    ) -> Result<NewSubscriber, SubscriberRepositoryError> {
        let id = self
            .get_subscriber_id_from_token(token)
            .await
            .map_err(|e| SubscriberRepositoryError::Unexpected(anyhow::Error::from(e)))?
            .ok_or_else(|| SubscriberRepositoryError::UnknownToken)?;

        let subscriber = self
            .get_subscriber_from_id(id)
            .await
            .context("Failed retrieving subscriber from a given subscriber id")?;

        Ok(subscriber)
    }
}
