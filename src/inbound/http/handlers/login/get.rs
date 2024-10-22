use crate::inbound::http::utils::{self, HtmlTemplate};
use actix_web::HttpResponse;
use actix_web_flash_messages::IncomingFlashMessages;

pub async fn login_form(flash_messages: IncomingFlashMessages) -> HttpResponse {
    let error_html = utils::flash_message_to_html(flash_messages);
    let html_content = utils::load_html(HtmlTemplate::Login);
    let page_content = html_content.replace("{error_html}", &error_html);

    utils::build_ok_html_response(page_content)
}
