use crate::inbound::http::auth::UserId;
use crate::inbound::http::config::get_template_path;
use crate::inbound::http::utils::e500;
use crate::{domain::auth::ports::AuthService, inbound::http::state::SharedAuthState};
use actix_web::http::header::ContentType;
use actix_web::{web, HttpResponse};
use std::fs;

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

    let path = get_template_path("dashboard.html");
    let html_content =
        fs::read_to_string(path).unwrap_or_else(|_| "Failed to load dashboard page".to_string());
    let page_content = html_content.replace("{username}", &username);
    Ok(HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(page_content))
}
