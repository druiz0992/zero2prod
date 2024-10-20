use crate::domain::auth::credentials::CredentialsError;
use crate::domain::newsletter::ports::NewsletterService;
use crate::inbound::http::{HmacSecret, NewsletterState};
use actix_web::error::InternalError;
use actix_web::http::header::LOCATION;
use actix_web::web;
use actix_web::HttpResponse;
use hmac::{Hmac, Mac};
use secrecy::ExposeSecret;

use crate::domain::auth::credentials::Credentials;

#[tracing::instrument(skip(credentials, state, secret))]
pub async fn login<NS: NewsletterService>(
    credentials: web::Form<Credentials>,
    state: web::Data<NewsletterState<NS>>,
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
    let hmac_tag = {
        let mut mac =
            Hmac::<sha2::Sha256>::new_from_slice(secret.0.expose_secret().as_bytes()).unwrap();
        mac.update(query_string.as_bytes());
        mac.finalize().into_bytes()
    };
    let response = HttpResponse::SeeOther()
        .insert_header((
            LOCATION,
            format!("/login?{}&tag={:x}", query_string, hmac_tag),
        ))
        .finish();
    Err(InternalError::from_response(error, response))
}
