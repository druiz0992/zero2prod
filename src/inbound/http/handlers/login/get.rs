use crate::inbound::http::config::get_template_path;
use actix_web::{http::header::ContentType, HttpResponse};
use actix_web_flash_messages::IncomingFlashMessages;
use std::fmt::Write;
use std::fs;

pub async fn login_form(flash_messages: IncomingFlashMessages) -> HttpResponse {
    let mut error_html = String::new();
    for m in flash_messages.iter() {
        writeln!(error_html, "<p><i>{}</i></p>", m.content()).unwrap();
    }

    let path = get_template_path("login.html");
    let html_content =
        fs::read_to_string(path).unwrap_or_else(|_| "Failed to load login page".to_string());
    let page_content = html_content.replace("{error_html}", &error_html);

    HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(page_content)
}
