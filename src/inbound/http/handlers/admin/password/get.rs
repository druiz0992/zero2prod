use crate::inbound::http::config::get_template_path;
use actix_web::http::header::ContentType;
use actix_web::HttpResponse;
use actix_web_flash_messages::IncomingFlashMessages;
use std::fmt::Write;
use std::fs;

pub async fn change_password_form(
    flash_message: IncomingFlashMessages,
) -> Result<HttpResponse, actix_web::Error> {
    let mut msg_html = String::new();
    for m in flash_message.iter() {
        writeln!(msg_html, "<p><i>{}</i></p>", m.content()).unwrap();
    }
    let path = get_template_path("change_password.html");
    let html_content = fs::read_to_string(path)
        .unwrap_or_else(|_| "Failed to load change password page".to_string());
    let page_content = html_content.replace("{msg_html}", &msg_html);

    Ok(HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(page_content))
}