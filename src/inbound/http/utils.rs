use actix_web::http::header::{ContentType, LOCATION};
use actix_web::HttpResponse;
use actix_web_flash_messages::IncomingFlashMessages;
use std::fmt::Write;
use std::fs;
use std::path::PathBuf;

pub fn e500<T>(e: T) -> actix_web::Error
where
    T: std::fmt::Debug + std::fmt::Display + 'static,
{
    actix_web::error::ErrorInternalServerError(e)
}

pub fn see_other(location: &str) -> HttpResponse {
    HttpResponse::SeeOther()
        .insert_header((LOCATION, location))
        .finish()
}

pub enum HtmlTemplate {
    ChangePassword,
    Dashboard,
    Home,
    Login,
    Newsletter,
}

const TEMPLATES_DIR: &str = "templates";

const TEMPLATE_CHANGE_PASSWORD: &str = "change_password.html";
const TEMPLATE_DASHBOARD: &str = "dashboard.html";
const TEMPLATE_HOME: &str = "home.html";
const TEMPLATE_LOGIN: &str = "login.html";
const TEMPLATE_NEWSLETTER: &str = "newsletter.html";

fn get_template_path(template: HtmlTemplate) -> (PathBuf, String) {
    let template_name = match template {
        HtmlTemplate::ChangePassword => TEMPLATE_CHANGE_PASSWORD,
        HtmlTemplate::Dashboard => TEMPLATE_DASHBOARD,
        HtmlTemplate::Home => TEMPLATE_HOME,
        HtmlTemplate::Login => TEMPLATE_LOGIN,
        HtmlTemplate::Newsletter => TEMPLATE_NEWSLETTER,
    };

    (
        PathBuf::from(TEMPLATES_DIR).join(template_name),
        template_name.to_string(),
    )
}

pub fn load_html(template: HtmlTemplate) -> String {
    let (path, name) = get_template_path(template);
    fs::read_to_string(path).unwrap_or_else(|_| format!("Failed to load {name} page"))
}

pub fn flash_message_to_html(flash_message: IncomingFlashMessages) -> String {
    let mut msg_html = String::new();
    for m in flash_message.iter() {
        writeln!(msg_html, "<p><i>{}</i></p>", m.content()).unwrap();
    }
    msg_html
}

pub fn build_ok_html_response(body: String) -> HttpResponse {
    HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(body)
}
