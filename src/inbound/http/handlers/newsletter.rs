#![allow(unused_imports)]

use uuid::Uuid;

use crate::{
    domain::{
        new_subscriber::models::{email::SubscriberEmail, token::SubscriptionToken},
        newsletter::{
            models::newsletter::{Newsletter, NewsletterBody, NewsletterDto},
            ports::NewsletterService,
        },
    },
    inbound::http::{auth::basic::basic_authentication, errors::AppError, SharedNewsletterState},
    outbound::telemetry::spawn_blocking_with_tracing,
};
use actix_web::{
    http::header::HeaderMap,
    {web, HttpRequest, HttpResponse, ResponseError},
};
use anyhow::Context;
use secrecy::{ExposeSecret, Secret};
use sqlx::PgPool;

#[tracing::instrument(
    name="Publish a newsletter issue",
    skip(body, state, request),
    fields(username=tracing::field::Empty, user_id=tracing::field::Empty, subscriber_email=tracing::field::Empty)
)]
pub async fn publish_newsletter<NS: NewsletterService>(
    body: web::Json<NewsletterDto>,
    state: web::Data<SharedNewsletterState<NS>>,
    request: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let newsletter = body.into_inner();
    let newsletter = newsletter.try_into()?;
    let credentials = basic_authentication(request)?;
    let base_url = &state.base_url;

    state
        .newsletter_service
        .validate_credentials(credentials)
        .await?;

    state
        .newsletter_service
        .send_newsletter(newsletter, base_url)
        .await?;

    Ok(HttpResponse::Ok().finish())
}
