use crate::{
    domain::new_subscriber::{
        models::subscriber::NewSubscriberRequest, ports::SubscriptionService,
    },
    inbound::http::{errors::AppError, SharedSubscriptionState},
};
use actix_web::{web, HttpResponse};

#[tracing::instrument(
    name = "Adding a new subscriber",
    skip(subscriber_request, state),
    fields(
        subscriber_email = %subscriber_request.email,
        subscriber_name = %subscriber_request.name,
    )
)]
pub async fn subscribe<SS: SubscriptionService>(
    subscriber_request: web::Form<NewSubscriberRequest>,
    state: web::Data<SharedSubscriptionState<SS>>,
) -> Result<HttpResponse, AppError> {
    let subscriber_request = subscriber_request.0;
    state
        .subscription_service
        .new_subscriber(subscriber_request)
        .await?;

    Ok(HttpResponse::Ok().finish())
}
