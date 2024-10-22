use crate::inbound::http::utils::{self, build_ok_html_response, HtmlTemplate};
use actix_web::HttpResponse;
use actix_web_flash_messages::IncomingFlashMessages;

pub async fn change_password_form(
    flash_message: IncomingFlashMessages,
) -> Result<HttpResponse, actix_web::Error> {
    let msg_html = utils::flash_message_to_html(flash_message);

    let html_content = utils::load_html(HtmlTemplate::ChangePassword);
    let page_content = html_content.replace("{msg_html}", &msg_html);

    Ok(build_ok_html_response(page_content))
}
