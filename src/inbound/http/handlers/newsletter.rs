#![allow(unused_imports)]

use uuid::Uuid;

use crate::domain::new_subscriber::models::email::SubscriberEmail;
use crate::domain::new_subscriber::models::token::SubscriptionToken;
use crate::domain::newsletter::models::newsletter::{Newsletter, NewsletterBody, NewsletterDto};
use crate::domain::newsletter::ports::NewsletterService;
use crate::inbound::http::auth;
use crate::inbound::http::{AppError, NewsletterState};
use crate::outbound::telemetry::spawn_blocking_with_tracing;
use actix_web::http::header::HeaderMap;
use actix_web::{web, HttpRequest, HttpResponse, ResponseError};
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
    state: web::Data<NewsletterState<NS>>,
    request: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let newsletter = body.into_inner();
    let newsletter = newsletter.try_into()?;
    let credentials = auth::basic_authentication(request)?;
    let base_url = &state.base_url;

    state
        .newsletter_service
        .send_newsletter(credentials, newsletter, base_url)
        .await?;

    Ok(HttpResponse::Ok().finish())
}
