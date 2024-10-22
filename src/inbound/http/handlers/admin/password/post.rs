use crate::domain::auth::credentials::{CredentialsError, PasswordChangeRequest};
use crate::domain::auth::ports::AuthService;
use crate::inbound::http::auth::UserId;
use crate::inbound::http::utils::{e500, see_other};
use crate::inbound::http::SharedAuthState;
use actix_web::{web, HttpResponse};
use actix_web_flash_messages::FlashMessage;

#[tracing::instrument(name = "Change password request", skip(req, user_id, state))]
pub async fn change_password<AS: AuthService>(
    state: web::Data<SharedAuthState<AS>>,
    req: web::Form<PasswordChangeRequest>,
    user_id: web::ReqData<UserId>,
) -> Result<HttpResponse, actix_web::Error> {
    let user_id = user_id.into_inner();
    let password_request = req.0;

    if !password_request.check() {
        FlashMessage::error(
            "You entered two different new passwords - the field values must match.",
        )
        .send();
        return Ok(see_other("/admin/password"));
    }

    let username = state
        .auth_service()
        .get_username(*user_id)
        .await
        .map_err(e500)?;

    let (old_credentials, new_credentials) = password_request.to_credentials(username);

    if let Err(e) = state
        .auth_service()
        .validate_credentials(old_credentials)
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

    state
        .auth_service()
        .change_password(*user_id, new_credentials.password())
        .await
        .map_err(e500)?;
    FlashMessage::error("Your password has been changed.").send();

    Ok(see_other("/admin/password"))
}
