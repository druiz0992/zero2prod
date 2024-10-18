use crate::domain::new_subscriber::errors::SubscriberError;
use async_trait::async_trait;

use super::*;

impl PostgresDb {
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

    #[tracing::instrument(name = "Get subscriber_id from token", skip(self, subscription_token))]
    pub async fn get_subscriber_id_from_token(
        &self,
        subscription_token: &SubscriptionToken,
    ) -> Result<SubscriberId, sqlx::Error> {
        let result = sqlx::query!(
            r#"SELECT subscriber_id FROM subscription_tokens WHERE subscription_token = $1"#,
            subscription_token.as_str(),
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.map(|r| r.subscriber_id))
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
            subscription_token.as_str(),
            subscriber_id.unwrap(),
        );
        transaction.execute(query).await?;
        Ok(())
    }

    #[tracing::instrument(
        name = "Delete token from subscriber id",
        skip(self, transaction, subscriber_id)
    )]
    async fn delete_subscriber_with_id(
        &self,
        transaction: &mut Transaction<'_, Postgres>,
        subscriber_id: uuid::Uuid,
    ) -> Result<(), SubscriberError> {
        let query = sqlx::query!(r#"DELETE FROM subscriptions WHERE  id= $1"#, subscriber_id);
        let result = transaction
            .execute(query)
            .await
            .map_err(|e| SubscriberError::Unexpected(anyhow::Error::from(e)))?;

        if result.rows_affected() == 0 {
            return Err(SubscriberError::NotFound(format!(
                "Subscriber with id {} not found",
                subscriber_id
            )));
        }

        Ok(())
    }

    #[tracing::instrument(
        name = "Delete subscriber with subscriber id",
        skip(self, transaction, subscriber_id)
    )]
    async fn delete_token_from_subscriber_id(
        &self,
        transaction: &mut Transaction<'_, Postgres>,
        subscriber_id: uuid::Uuid,
    ) -> Result<(), SubscriberError> {
        let query = sqlx::query!(
            r#"DELETE FROM subscription_tokens WHERE  subscriber_id= $1"#,
            subscriber_id
        );
        let result = transaction
            .execute(query)
            .await
            .map_err(|e| SubscriberError::Unexpected(anyhow::Error::from(e)))?;

        if result.rows_affected() == 0 {
            return Err(SubscriberError::AuthError(format!(
                "Subscriber with id {} couldnt be authenticated",
                subscriber_id
            )));
        }

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
    ) -> Result<(NewSubscriber, SubscriptionToken), SubscriberError> {
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
    async fn update(&self, subscriber: NewSubscriber) -> Result<(), SubscriberError> {
        let result = sqlx::query!(
            r#"UPDATE subscriptions SET email = $1, name = $2, status = $3 WHERE id = $4"#,
            subscriber.email.as_str(),
            subscriber.name.as_str(),
            String::from(subscriber.status),
            subscriber.id,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| SubscriberError::Unexpected(anyhow::Error::from(e)))
        .context("Failed to update subscriber to database")?;

        if result.rows_affected() == 0 {
            return Err(SubscriberError::NotFound(format!(
                "Subscriber with email {} not found,",
                subscriber.email.as_str(),
            )));
        }

        Ok(())
    }

    #[tracing::instrument(name = "Retrieve subscriber from token", skip(token, self))]
    async fn retrieve_from_token(
        &self,
        token: &SubscriptionToken,
    ) -> Result<NewSubscriber, SubscriberError> {
        let id = self
            .get_subscriber_id_from_token(token)
            .await
            .map_err(|e| SubscriberError::Unexpected(anyhow::Error::from(e)))?
            .ok_or_else(|| SubscriberError::AuthError("Token not found".to_string()))?;

        let subscriber = self
            .get_subscriber_from_id(id)
            .await
            .context("Failed retrieving subscriber from a given subscriber id")?;

        Ok(subscriber)
    }

    #[tracing::instrument(name = "Deleting subscriber", skip(subscriber, self))]
    async fn delete(&self, subscriber: NewSubscriber) -> Result<(), SubscriberError> {
        let mut transaction = self
            .pool
            .begin()
            .await
            .context("Failed to acquire a Postgress connection from the pool")?;
        let subscriber_id = subscriber.id.unwrap();
        self.delete_token_from_subscriber_id(&mut transaction, subscriber_id)
            .await?;
        self.delete_subscriber_with_id(&mut transaction, subscriber_id)
            .await?;
        transaction
            .commit()
            .await
            .context("Failed to commit SQL transaction to delete a subscriber")?;

        Ok(())
    }
}
