mod errors;
use errors::*;

use crate::domain::new_subscriber::models::token::SubscriptionToken;
use crate::outbound::db::postgres_db::PostgresDb;
use actix_web::{web, HttpResponse};
use anyhow::Context;

#[derive(serde::Deserialize)]
pub struct Parameters {
    subscription_token: String,
}

impl TryFrom<Parameters> for SubscriptionToken {
    type Error = String;
    fn try_from(value: Parameters) -> Result<Self, Self::Error> {
        let subscription_token = SubscriptionToken::parse(value.subscription_token)?;
        Ok(subscription_token)
    }
}

#[tracing::instrument(name = "Confirm a pending subscriber", skip(parameters, pool))]
pub async fn confirm(
    parameters: web::Query<Parameters>,
    pool: web::Data<PostgresDb>,
) -> Result<HttpResponse, ConfirmationError> {
    let subscription_token = parameters
        .0
        .try_into()
        .map_err(ConfirmationError::ValidationError)?;
    let subscriber_id = pool
        .get_subscriber_id_from_token(&subscription_token)
        .await
        .context("Failed retrieving a subscriber id associated with provided token")?
        .ok_or(ConfirmationError::UnknownToken)?;

    pool.confirm_subscriber(subscriber_id)
        .await
        .context("Failed updating user status in db")?;
    Ok(HttpResponse::Ok().finish())
}
