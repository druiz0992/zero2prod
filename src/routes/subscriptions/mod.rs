mod errors;

use errors::*;

use crate::routes::subscriptions::SubscriberError;
use crate::{
    domain::{NewSubscriber, SubscriberEmail, SubscriberName},
    email_client::EmailClient,
};
use actix_web::{web, HttpResponse};
use anyhow::Context;
use chrono::Utc;
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use sqlx::{Executor, PgPool, Postgres, Transaction};
use uuid::Uuid;

#[derive(serde::Deserialize)]
pub struct FormData {
    email: String,
    name: String,
}

impl TryFrom<FormData> for NewSubscriber {
    type Error = String;
    fn try_from(value: FormData) -> Result<Self, Self::Error> {
        let name = SubscriberName::parse(value.name)?;
        let email = SubscriberEmail::parse(value.email)?;
        Ok(Self { email, name })
    }
}

#[tracing::instrument(
    name = "Adding a new subscriber",
    skip(form, pool, email_client, base_url),
    fields(
        subscriber_email = %form.email,
        subscriber_name = %form.name
    )
)]
pub async fn subscribe(
    form: web::Form<FormData>,
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
    base_url: web::Data<String>,
) -> Result<HttpResponse, SubscriberError> {
    let subscription_token: String;
    let new_subscriber = form
        .0
        .try_into()
        .map_err(SubscriberError::ValidationError)?;

    if let Some(subscriber_id) = get_subscriber_id_if_pending_confirmation(&pool, &new_subscriber)
        .await
        .context("Failed to check if subscriber existed in db.")?
    {
        subscription_token = get_token_from_subscriber_id(&pool, &subscriber_id)
            .await
            .context("Failed to read token from database")?;
    } else {
        let mut transaction = pool
            .begin()
            .await
            .context("Failed to acquire a Postgress connection from the pool")?;

        let subscriber_id = insert_subscriber(&mut transaction, &new_subscriber)
            .await
            .context("Failed to insert a new subscriber in the database")?;
        subscription_token = generate_subscription_token();
        store_token(&mut transaction, subscriber_id, &subscription_token)
            .await
            .context("Failed to store the confirmation token for a new subscriber.")?;

        transaction
            .commit()
            .await
            .context("Failed to commit SQL transaction to store a new subscriber")?;
    }

    send_confirmation_email(
        &email_client,
        new_subscriber,
        &base_url,
        &subscription_token,
    )
    .await
    .context("Failed to send a confirmation email")?;

    Ok(HttpResponse::Ok().finish())
}

#[tracing::instrument(
    name = "Checking if user is already subscribed",
    skip(pool, subscriber)
)]
pub async fn get_subscriber_id_if_pending_confirmation(
    pool: &PgPool,
    subscriber: &NewSubscriber,
) -> Result<Option<Uuid>, sqlx::Error> {
    let result = sqlx::query!(
        r#"SELECT id, status FROM subscriptions WHERE email = $1"#,
        subscriber.email.as_ref(),
    )
    .fetch_optional(pool)
    .await?;

    Ok(result
        .filter(|r| r.status == "pending_confirmation")
        .map(|r| r.id))
}

#[tracing::instrument(
    name = "Store subscription token in the database",
    skip(subscription_token, transaction)
)]
pub async fn store_token(
    transaction: &mut Transaction<'_, Postgres>,
    subscriber_id: Uuid,
    subscription_token: &str,
) -> Result<(), sqlx::Error> {
    let query = sqlx::query!(
        r#"INSERT INTO subscription_tokens (subscription_token, subscriber_id)
        VALUES ($1, $2)"#,
        subscription_token,
        subscriber_id
    );
    transaction.execute(query).await?;
    Ok(())
}

#[tracing::instrument(
    name = "Send a confirmation email to a new subscriber",
    skip(email_client, new_subscriber, base_url)
)]
pub async fn send_confirmation_email(
    email_client: &EmailClient,
    new_subscriber: NewSubscriber,
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
        .send_email(new_subscriber.email, "Welcome", html_body, plain_body)
        .await
}

#[tracing::instrument(
    name = "Saving new subscriber details in db",
    skip(new_subscriber, transaction)
)]
pub async fn insert_subscriber(
    transaction: &mut Transaction<'_, Postgres>,
    new_subscriber: &NewSubscriber,
) -> Result<Uuid, sqlx::Error> {
    let subscriber_id = Uuid::new_v4();
    let query = sqlx::query!(
        r#"
    INSERT INTO subscriptions (id, email, name, subscribed_at, status)
    VALUES ($1, $2, $3, $4, 'pending_confirmation')
            "#,
        subscriber_id,
        new_subscriber.email.as_ref(),
        new_subscriber.name.as_ref(),
        Utc::now(),
    );
    transaction.execute(query).await?;

    Ok(subscriber_id)
}

fn generate_subscription_token() -> String {
    let mut rng = thread_rng();
    std::iter::repeat_with(|| rng.sample(Alphanumeric))
        .map(char::from)
        .take(25)
        .collect()
}

#[tracing::instrument(name = "Get token from subscriber id", skip(pool, subscriber_id))]
pub async fn get_token_from_subscriber_id(
    pool: &PgPool,
    subscriber_id: &Uuid,
) -> Result<String, sqlx::Error> {
    let result = sqlx::query!(
        r#"SELECT subscription_token FROM subscription_tokens WHERE  subscriber_id= $1"#,
        subscriber_id
    )
    .fetch_one(pool)
    .await?;

    Ok(result.subscription_token)
}
