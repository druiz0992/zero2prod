use crate::domain::new_subscriber::{
    models::token::SubscriptionTokenRequest, ports::SubscriptionService,
};
use crate::inbound::http::{AppError, ApplicationState};
use actix_web::{web, HttpResponse};

#[tracing::instrument(name = "Confirm a pending subscriber", skip(req, state))]
pub async fn confirm<SS: SubscriptionService>(
    req: web::Query<SubscriptionTokenRequest>,
    state: web::Data<ApplicationState<SS>>,
) -> Result<HttpResponse, AppError> {
    let req = req.into_inner();
    state.subscription_service.confirm(req).await?;
    Ok(HttpResponse::Ok().finish())
}
