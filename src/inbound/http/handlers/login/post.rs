use crate::domain::auth::credentials::CredentialsError;
use crate::domain::auth::ports::AuthService;
use crate::inbound::http::auth::secure_query::SecureQuery;
use crate::inbound::http::{HmacSecret, SharedAuthState};
use actix_web::error::InternalError;
use actix_web::http::header::LOCATION;
use actix_web::web;
use actix_web::HttpResponse;

use crate::domain::auth::credentials::Credentials;

#[tracing::instrument(skip(credentials, state, secret))]
pub async fn login<AS: AuthService>(
    credentials: web::Form<Credentials>,
    state: web::Data<SharedAuthState<AS>>,
    secret: web::Data<HmacSecret>,
) -> Result<HttpResponse, InternalError<CredentialsError>> {
    let credentials = credentials.0;

    match state.auth_service().validate_credentials(credentials).await {
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
    let secure_query = SecureQuery::new(error.to_string(), secret.as_ref());
    let response = HttpResponse::SeeOther()
        .insert_header((
            LOCATION,
            format!("/login?{}&tag={}", secure_query.query(), secure_query.tag(),),
        ))
        .finish();
    Err(InternalError::from_response(error, response))
}
