mod errors;

use errors::*;
use uuid::Uuid;

use crate::domain::{Newsletter, NewsletterBody};
use crate::routes::get_token_from_subscriber_id;
use crate::telemetry::spawn_blocking_with_tracing;
use crate::{domain::SubscriberEmail, email_client::EmailClient};
use actix_web::http::header::HeaderMap;
use actix_web::{web, HttpRequest, HttpResponse};
use anyhow::Context;
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use secrecy::{ExposeSecret, Secret};
use sqlx::PgPool;

#[derive(serde::Deserialize)]
pub struct BodyData {
    title: String,
    content: Content,
}

#[derive(serde::Deserialize)]
pub struct Content {
    html: String,
    text: String,
}

struct ConfirmedSubscriber {
    email: SubscriberEmail,
}

struct Credentials {
    username: String,
    password: Secret<String>,
}

impl TryFrom<BodyData> for Newsletter {
    type Error = String;

    fn try_from(body: BodyData) -> Result<Self, Self::Error> {
        let newsletter = Newsletter::parse(body.title, body.content.html, body.content.text)?;
        Ok(newsletter)
    }
}

#[tracing::instrument(
    name="Publish a newsletter issue",
    skip(body, pool, email_client, request),
    fields(username=tracing::field::Empty, user_id=tracing::field::Empty, subscriber_email=tracing::field::Empty)
)]
pub async fn publish_newsletter(
    body: web::Json<BodyData>,
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
    base_url: web::Data<String>,
    request: HttpRequest,
) -> Result<HttpResponse, PublishError> {
    let newsletter: Newsletter = body
        .0
        .try_into()
        .map_err(|e| PublishError::ValidationError(e))?;
    let credentials = basic_authentication(request.headers()).map_err(PublishError::AuthError)?;
    tracing::Span::current().record("username", tracing::field::display(&credentials.username));
    let user_id = validate_credentials(credentials, &pool).await?;
    tracing::Span::current().record("user_id", tracing::field::display(&user_id));
    let subscribers = get_confirmed_subscribers(&pool).await?;
    for subscriber in subscribers {
        match subscriber {
            Ok(subscriber) => {
                tracing::Span::current().record(
                    "subscriber_email",
                    tracing::field::display(&subscriber.email),
                );
                let subscriber_id = get_subscriber_id_from_email(&pool, subscriber.email.as_ref())
                    .await
                    .context("Failed to retreive subscriber id from database")?;
                let subscription_token = get_token_from_subscriber_id(&pool, &subscriber_id)
                    .await
                    .context("Failed to read unsubscribe token from database.")?;
                let unsubscribe_link = build_unsubscribe_link(&base_url, &subscription_token);
                email_client
                    .send_email(
                        &subscriber.email,
                        newsletter.title.as_ref(),
                        &embed_link_to_html_content(&newsletter.content.html, &unsubscribe_link),
                        &embed_link_to_text_content(&newsletter.content.text, &unsubscribe_link),
                    )
                    .await
                    .with_context(|| {
                        format!("Failed to send newsletter issue to {}", subscriber.email)
                    })?;
            }
            Err(error) => {
                tracing::warn!(
                    error.cause_chain = ?error,
                    "Skipping a confirmed subscriber. Their stored contact details are invalid",
                );
            }
        }
    }
    Ok(HttpResponse::Ok().finish())
}

fn embed_link_to_text_content(body: &NewsletterBody, link: &str) -> String {
    let text_with_link = format!(
        "\nClick <a href=\"{}\">here</a> to unsubscribe from newsletter.",
        link
    );
    let content_with_link = format!("{} {} ", body.as_ref(), text_with_link);
    content_with_link
}
fn embed_link_to_html_content(body: &NewsletterBody, link: &str) -> String {
    let text_with_link = format!("\nClick here {} to unsubscribe from newsletter.", link);
    let content_with_link = format!("{} {} ", body.as_ref(), text_with_link);
    content_with_link
}

fn build_unsubscribe_link(base_url: &str, token: &str) -> String {
    let unsubscribe_link = format!("{}/subscriptions/unsubscribe?token={}", base_url, token);
    unsubscribe_link
}

#[tracing::instrument(name = "Get subscriber id", skip(pool, email))]
async fn get_subscriber_id_from_email(pool: &PgPool, email: &str) -> Result<Uuid, anyhow::Error> {
    let subscriber_id = sqlx::query!("SELECT id from subscriptions WHERE email=$1", email)
        .fetch_one(pool)
        .await?;

    Ok(subscriber_id.id)
}

#[tracing::instrument(name = "Get confirmed subscribers", skip(pool))]
async fn get_confirmed_subscribers(
    pool: &PgPool,
) -> Result<Vec<Result<ConfirmedSubscriber, anyhow::Error>>, anyhow::Error> {
    let confirmed_subscribers =
        sqlx::query!(r#"SELECT email FROM subscriptions WHERE status = 'confirmed'"#)
            .fetch_all(pool)
            .await?
            .into_iter()
            .map(|r| match SubscriberEmail::parse(r.email) {
                Ok(email) => Ok(ConfirmedSubscriber { email }),
                Err(error) => Err(anyhow::anyhow!(error)),
            })
            .collect();
    Ok(confirmed_subscribers)
}

fn basic_authentication(headers: &HeaderMap) -> Result<Credentials, anyhow::Error> {
    let header_value = headers
        .get("Authorization")
        .context("The 'Authorization' header is missing")?
        .to_str()
        .context("The 'Authorization' header is not a valid UTF8 String.")?;
    let base64encoded_segment = header_value
        .strip_prefix("Basic ")
        .context("The authorization scheme was no 'Basic'")?;
    let decoded_bytes = base64::decode_config(base64encoded_segment, base64::STANDARD)
        .context("Failed to base64-decode 'Basic' credentials")?;
    let decoded_credentials = String::from_utf8(decoded_bytes)
        .context("The decided credential string is not valid UTF8.")?;

    let mut credentials = decoded_credentials.splitn(2, ':');
    let username = credentials
        .next()
        .ok_or_else(|| anyhow::anyhow!("A username must be provided in 'Basic' auth."))?
        .to_string();
    let password = credentials
        .next()
        .ok_or_else(|| anyhow::anyhow!("A password must be provided in 'Basic' auth."))?
        .to_string();

    Ok(Credentials {
        username,
        password: Secret::new(password),
    })
}

#[tracing::instrument(name = "Validate credentials", skip(credentials, pool))]
async fn validate_credentials(
    credentials: Credentials,
    pool: &PgPool,
) -> Result<uuid::Uuid, PublishError> {
    let mut user_id = None;
    let mut expected_password_hash = Secret::new(
        "$argon2id$v=19$m=15000,t=2,p=1$\
gZiV/M1gPc22ElAH/Jh1Hw$\
CWOrkoo7oJBQ/iyh7uJ0LO2aLEfrHwTWllSAxT0zRno"
            .to_string(),
    );

    if let Some((stored_user_id, stored_password_hash)) =
        get_stored_credentials(&credentials.username, pool)
            .await
            .map_err(PublishError::UnexpectedError)?
    {
        user_id = Some(stored_user_id);
        expected_password_hash = stored_password_hash;
    }

    spawn_blocking_with_tracing(move || {
        verify_password_hash(expected_password_hash, credentials.password)
    })
    .await
    .context("Failed to spawn a blocking task.")
    .map_err(PublishError::UnexpectedError)??;

    user_id.ok_or_else(|| PublishError::AuthError(anyhow::anyhow!("Unknown username.")))
}

#[tracing::instrument(
    name = "Verify password hash",
    skip(expected_password_hash, password_candidate)
)]
fn verify_password_hash(
    expected_password_hash: Secret<String>,
    password_candidate: Secret<String>,
) -> Result<(), PublishError> {
    let expected_password_hash = PasswordHash::new(expected_password_hash.expose_secret())
        .context("Failed to parse hash in PHC string format.")
        .map_err(PublishError::UnexpectedError)?;

    Argon2::default()
        .verify_password(
            password_candidate.expose_secret().as_bytes(),
            &expected_password_hash,
        )
        .context("Invalid password.")
        .map_err(PublishError::AuthError)
}

#[tracing::instrument(name = "Get stored credentials", skip(username, pool))]
async fn get_stored_credentials(
    username: &str,
    pool: &PgPool,
) -> Result<Option<(uuid::Uuid, Secret<String>)>, anyhow::Error> {
    let row = sqlx::query!(
        r#"SELECT user_id, password_hash FROM users WHERE username = $1"#,
        username,
    )
    .fetch_optional(pool)
    .await
    .context("Failed to perform a query to retrieve stored credentials.")?
    .map(|row| (row.user_id, Secret::new(row.password_hash)));
    Ok(row)
}
