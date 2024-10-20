use crate::{
    domain::new_subscriber::{models::token::SubscriptionTokenRequest, ports::SubscriptionService},
    inbound::http::{errors::AppError, SharedSubscriptionState},
};
use actix_web::{web, HttpResponse};

#[tracing::instrument(name = "Confirm a pending subscriber", skip(req, state))]
pub async fn confirm<SS: SubscriptionService>(
    req: web::Query<SubscriptionTokenRequest>,
    state: web::Data<SharedSubscriptionState<SS>>,
) -> Result<HttpResponse, AppError> {
    let req = req.into_inner();
    state.subscription_service.confirm(req).await?;
    Ok(HttpResponse::Ok().finish())
}
