use crate::configuration::DatabaseSettings;
use crate::domain::new_subscriber::models::name::SubscriberName;
use crate::domain::new_subscriber::models::subscriber::NewSubscriberRequest;
use crate::domain::new_subscriber::models::subscriber::{
    NewSubscriberError, SubscriberId, SubscriberStatus,
};
use crate::domain::new_subscriber::models::{
    email::SubscriberEmail, subscriber::NewSubscriber, token::SubscriptionToken,
};
use crate::domain::new_subscriber::ports::SubscriptionRepository;
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

    #[tracing::instrument(
        name = "Checking if user is already subscribed",
        skip(self, subscriber)
    )]
    async fn get_subscriber(
        &self,
        subscriber: NewSubscriber,
    ) -> Result<NewSubscriber, anyhow::Error> {
        let record = sqlx::query!(
            "SELECT id, email, name, status FROM subscriptions WHERE email = $1",
            subscriber.email.as_ref()
        )
        .fetch_optional(&self.pool)
        .await
        .expect("Failed to fetch saved subscription.");

        let (id, status) = match record {
            Some(existing_subscriber) => (
                Some(existing_subscriber.id),
                SubscriberStatus::parse(&existing_subscriber.status).unwrap(),
            ),

            None => (None, SubscriberStatus::NotInserted),
        };

        Ok(NewSubscriber {
            id,
            email: subscriber.email,
            name: subscriber.name,
            status,
        })
    }

    // TODO: This is only for testing
    pub async fn drop_column(&self, column: &str) {
        if column == "email" {
            sqlx::query!("ALTER TABLE subscriptions DROP COLUMN email;",)
                .execute(&self.pool)
                .await
                .unwrap();
        } else {
            sqlx::query!("ALTER TABLE subscription_tokens DROP COLUMN subscription_token;",)
                .execute(&self.pool)
                .await
                .unwrap();
        }
    }

    #[tracing::instrument(
        name = "Saving new subscriber details in db",
        skip(new_subscriber, transaction)
    )]
    async fn insert_subscriber(
        &self,
        transaction: &mut Transaction<'_, Postgres>,
        new_subscriber: &NewSubscriber,
    ) -> Result<NewSubscriber, sqlx::Error> {
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
        transaction.execute(query).await?;

        let new_subscriber = NewSubscriber {
            id: Some(subscriber_id),
            email: new_subscriber.email.clone(),
            name: new_subscriber.name.clone(),
            status: SubscriberStatus::SubscriptionPendingConfirmation,
        };

        Ok(new_subscriber)
    }

    #[tracing::instrument(
        name = "Store subscription token in the database",
        skip(subscription_token, transaction)
    )]
    async fn store_token(
        &self,
        transaction: &mut Transaction<'_, Postgres>,
        subscriber_id: SubscriberId,
        subscription_token: &SubscriptionToken,
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
    ) -> Result<SubscriptionToken, anyhow::Error> {
        let result = sqlx::query!(
            r#"SELECT subscription_token FROM subscription_tokens WHERE  subscriber_id= $1"#,
            subscriber_id
        )
        .fetch_one(&self.pool)
        .await?;

        SubscriptionToken::try_from(result.subscription_token)
            .map_err(|e| anyhow::Error::msg(format!("{}", e)))
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
impl SubscriptionRepository for PostgresDb {
    #[tracing::instrument(
        name = "Adding a new subscriber",
        skip(self, subscriber_request, token)
    )]
    async fn retrieve_or_insert(
        &self,
        subscriber_request: NewSubscriberRequest,
        token: SubscriptionToken,
    ) -> Result<(NewSubscriber, SubscriptionToken), anyhow::Error> {
        let mut new_token = token;
        let mut new_subscriber: NewSubscriber = subscriber_request
            .try_into()
            .map_err(|_| NewSubscriberError::InvalidEmail("WWW".into()))?;
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
                .insert_subscriber(&mut transaction, &new_subscriber)
                .await
                .context("Failed to insert a new subscriber in the database")?;
            self.store_token(&mut transaction, new_subscriber.id, &new_token)
                .await
                .context("Failed to store the confirmation token for a new subscriber.")?;

            transaction
                .commit()
                .await
                .context("Failed to commit SQL transaction to store a new subscriber")?;
        }
        Ok((new_subscriber, new_token))
    }
}
