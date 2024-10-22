use crate::inbound::http::auth::UserId;
use crate::inbound::http::utils::{self, e500, HtmlTemplate};
use crate::{domain::auth::ports::AuthService, inbound::http::state::SharedAuthState};
use actix_web::{web, HttpResponse};

#[tracing::instrument(name = "Admin dashboard", skip(user_id, state))]
pub async fn admin_dashboard<AS: AuthService>(
    state: web::Data<SharedAuthState<AS>>,
    user_id: web::ReqData<UserId>,
) -> Result<HttpResponse, actix_web::Error> {
    let user_id = user_id.into_inner();
    let username = state
        .auth_service()
        .get_username(*user_id)
        .await
        .map_err(e500)?;

    let html_content = utils::load_html(HtmlTemplate::Dashboard);
    let page_content = html_content.replace("{username}", &username);

    Ok(utils::build_ok_html_response(page_content))
}
