mod errors;

use errors::*;

use crate::domain::new_subscriber::models::subscriber::{NewSubscriber, SubscriberStatus};
use crate::domain::new_subscriber::models::token::SubscriptionToken;
use crate::domain::new_subscriber::ports::{SubscriptionRepository, SubscriptionService};
use crate::email_client;
use crate::routes::subscriptions::SubscriberError;
use crate::{
    domain::new_subscriber::models::subscriber::NewSubscriberRequest, email_client::EmailClient,
};
use actix_web::{web, HttpResponse};
use anyhow::Context;

#[tracing::instrument(
    name = "Adding a new subscriber",
    skip(subscriber_request, subscription_service, email_client, base_url),
    fields(
        subscriber_email = %subscriber_request.email,
        subscriber_name = %subscriber_request.name,
    )
)]
pub async fn subscribe(
    subscriber_request: web::Form<NewSubscriberRequest>,
    subscription_service: web::Data<Box<dyn SubscriptionService>>,
    email_client: web::Data<EmailClient>,
    base_url: web::Data<String>,
) -> Result<HttpResponse, SubscriberError> {
    let subscriber_request = subscriber_request.0;
    subscription_service
        .new_subscriber(subscriber_request, &email_client, &base_url)
        .await
        .map_err(SubscriberError::UnexpectedError)?;

    Ok(HttpResponse::Ok().finish())
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
        .send_email(&new_subscriber.email, "Welcome", html_body, plain_body)
        .await
}
