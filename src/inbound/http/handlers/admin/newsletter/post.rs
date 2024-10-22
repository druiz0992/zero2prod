use crate::inbound::http::utils::see_other;
use crate::{
    domain::newsletter::{models::newsletter::NewsletterDto, ports::NewsletterService},
    inbound::http::{errors::AppError, SharedNewsletterState},
};
use actix_web_flash_messages::FlashMessage;

use actix_web::{web, HttpResponse};

#[tracing::instrument(
    name="Publish a newsletter issue",
    skip(body, state),
    fields(username=tracing::field::Empty, user_id=tracing::field::Empty, subscriber_email=tracing::field::Empty)
)]
pub async fn publish_newsletter<NS: NewsletterService>(
    body: web::Form<NewsletterDto>,
    state: web::Data<SharedNewsletterState<NS>>,
) -> Result<HttpResponse, AppError> {
    let newsletter = body.into_inner();
    let newsletter = newsletter.try_into()?;
    let base_url = state.url();

    state
        .newsletter_service()
        .send_newsletter(newsletter, base_url)
        .await?;

    FlashMessage::info("The newsletter issue has been published!").send();
    Ok(see_other("/admin/newsletters"))
}
