use crate::domain::auth::credentials::CredentialsError;
use crate::domain::auth::ports::AuthService;
use crate::inbound::http::auth::session::TypedSession;
use crate::inbound::http::SharedAuthState;
use actix_web::error::InternalError;
use actix_web::http::header::LOCATION;
use actix_web::web;
use actix_web::HttpResponse;
use actix_web_flash_messages::FlashMessage;

use crate::domain::auth::credentials::Credentials;

#[tracing::instrument(name = "login request", skip(credentials, state, session))]
pub async fn login<AS: AuthService>(
    credentials: web::Form<Credentials>,
    state: web::Data<SharedAuthState<AS>>,
    session: TypedSession,
) -> Result<HttpResponse, InternalError<CredentialsError>> {
    let credentials = credentials.0;

    match state.auth_service().validate_credentials(credentials).await {
        Ok(user_id) => {
            session.renew();
            session.insert_user_id(user_id).map_err(|e| {
                handle_login_failure(CredentialsError::Unexpected(e.into())).unwrap_err()
            })?;
            handle_login_success()
        }
        Err(error) => handle_login_failure(error),
    }
}

fn handle_login_success() -> Result<HttpResponse, InternalError<CredentialsError>> {
    Ok(HttpResponse::SeeOther()
        .insert_header((LOCATION, "/admin/dashboard"))
        .finish())
}

fn handle_login_failure(
    error: CredentialsError,
) -> Result<HttpResponse, InternalError<CredentialsError>> {
    FlashMessage::error(error.to_string()).send();
    let response = HttpResponse::SeeOther()
        .insert_header((LOCATION, "/login".to_string()))
        .finish();
    Err(InternalError::from_response(error, response))
}
