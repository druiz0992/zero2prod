use crate::domain::auth::credentials::CredentialsError;
use crate::domain::newsletter::ports::NewsletterService;
use crate::inbound::http::auth::secure_query::SecureQuery;
use crate::inbound::http::{HmacSecret, SharedNewsletterState};
use actix_web::error::InternalError;
use actix_web::http::header::LOCATION;
use actix_web::web;
use actix_web::HttpResponse;

use crate::domain::auth::credentials::Credentials;

#[tracing::instrument(skip(credentials, state, secret))]
pub async fn login<NS: NewsletterService>(
    credentials: web::Form<Credentials>,
    state: web::Data<SharedNewsletterState<NS>>,
    secret: web::Data<HmacSecret>,
) -> Result<HttpResponse, InternalError<CredentialsError>> {
    let credentials = credentials.0;

    match state
        .newsletter_service
        .validate_credentials(credentials)
        .await
    {
        Ok(_) => handle_login_success(),
        Err(error) => handle_login_failure(error, secret),
    }
}

fn handle_login_success() -> Result<HttpResponse, InternalError<CredentialsError>> {
    Ok(HttpResponse::SeeOther()
        .insert_header((LOCATION, "/"))
        .finish())
}

fn handle_login_failure(
    error: CredentialsError,
    secret: web::Data<HmacSecret>,
) -> Result<HttpResponse, InternalError<CredentialsError>> {
    let query_string = format!("error={}", urlencoding::Encoded::new(error.to_string()));
    let secure_query = SecureQuery::new(query_string, secret.as_ref());
    let response = HttpResponse::SeeOther()
        .insert_header((
            LOCATION,
            format!("/login?{}&tag={}", secure_query.query(), secure_query.tag(),),
        ))
        .finish();
    Err(InternalError::from_response(error, response))
}
