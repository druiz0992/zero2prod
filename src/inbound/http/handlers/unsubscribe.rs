use crate::{
    domain::new_subscriber::{models::token::SubscriptionTokenRequest, ports::SubscriptionService},
    inbound::http::{errors::AppError, SharedSubscriptionState},
};
use actix_web::{web, HttpResponse};

#[tracing::instrument(name = "Deleting a subscriber", skip(state, req))]
pub async fn unsubscribe<SS: SubscriptionService>(
    state: web::Data<SharedSubscriptionState<SS>>,
    req: web::Query<SubscriptionTokenRequest>,
) -> Result<HttpResponse, AppError> {
    let req = req.into_inner();
    state.subscription_service.delete(req).await?;
    Ok(HttpResponse::Ok().finish())
}
