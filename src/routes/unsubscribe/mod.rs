mod errors;

use crate::domain::SubscriptionToken;
use actix_web::{web, HttpResponse};
use errors::*;
use sqlx::PgPool;

#[derive(serde::Deserialize)]
pub struct UnsubscribeParameters {
    token: String,
}

impl TryFrom<UnsubscribeParameters> for SubscriptionToken {
    type Error = String;
    fn try_from(value: UnsubscribeParameters) -> Result<Self, Self::Error> {
        let token = SubscriptionToken::parse(value.token)?;
        Ok(token)
    }
}
#[tracing::instrument(name = "Removing asubscriber", skip(_pool, parameters))]
pub async fn unsubscribe(
    _pool: web::Data<PgPool>,
    parameters: web::Query<UnsubscribeParameters>,
) -> Result<HttpResponse, UnsubscribeError> {
    let _unsubscribe_token: SubscriptionToken = parameters
        .0
        .try_into()
        .map_err(UnsubscribeError::ValidationError)?;
    // retrieve subscriber from token. If it doesnt exist (token or subscriber), raise error
    // render HTML page confirming unsubscription that redirects to DELETE /subsriptions
    Ok(HttpResponse::Ok().finish())
}
