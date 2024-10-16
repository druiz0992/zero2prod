use crate::domain::new_subscriber::{
    models::token::SubscriptionTokenRequest, ports::SubscriptionService,
};
use crate::inbound::http::{AppError, ApplicationState};
use actix_web::{web, HttpResponse};

#[tracing::instrument(name = "Removing a subscriber", skip(state, req))]
pub async fn unsubscribe<SS: SubscriptionService>(
    state: web::Data<ApplicationState<SS>>,
    req: web::Query<SubscriptionTokenRequest>,
) -> Result<HttpResponse, AppError> {
    let req = req.into_inner();
    state.subscription_service.delete(req).await?;
    Ok(HttpResponse::Ok().finish())
}
