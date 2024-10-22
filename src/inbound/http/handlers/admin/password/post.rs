use crate::domain::auth::credentials::{Credentials, CredentialsError};
use crate::domain::auth::ports::AuthService;
use crate::inbound::http::auth::UserId;
use crate::inbound::http::utils::{e500, see_other};
use crate::inbound::http::SharedAuthState;
use actix_web::{web, HttpResponse};
use actix_web_flash_messages::FlashMessage;
use secrecy::{ExposeSecret, Secret};

#[derive(serde::Deserialize)]
pub struct FormData {
    current_password: Secret<String>,
    new_password: Secret<String>,
    new_password_check: Secret<String>,
}
#[tracing::instrument(name = "Change password request", skip(form, user_id, state))]
pub async fn change_password<AS: AuthService>(
    state: web::Data<SharedAuthState<AS>>,
    form: web::Form<FormData>,
    user_id: web::ReqData<UserId>,
) -> Result<HttpResponse, actix_web::Error> {
    let user_id = user_id.into_inner();
    let form = form.0;

    if !new_password_check(&form) {
        return Ok(see_other("/admin/password"));
    }

    let username = state
        .auth_service()
        .get_username(*user_id)
        .await
        .map_err(e500)?;
    let credentials = Credentials::new(
        username.clone(),
        form.current_password.expose_secret().to_string(),
    );

    if let Err(e) = state
        .auth_service()
        .validate_credentials(credentials.clone())
        .await
    {
        return match e {
            CredentialsError::AuthError(_) => {
                FlashMessage::error("The current password is incorrect.").send();
                Ok(see_other("/admin/password"))
            }
            CredentialsError::Unexpected(_) => Err(e500(e)),
        };
    }
    let new_credentials = Credentials::new(username, form.new_password.expose_secret().to_string());

    state
        .auth_service()
        .change_password(*user_id, new_credentials.password())
        .await
        .map_err(e500)?;
    FlashMessage::error("Your password has been changed.").send();

    Ok(see_other("/admin/password"))
}

fn new_password_check(form: &FormData) -> bool {
    if form.new_password.expose_secret() != form.new_password_check.expose_secret() {
        FlashMessage::error(
            "You entered two different new passwords - the field values must match.",
        )
        .send();
        return false;
    }
    true
}
