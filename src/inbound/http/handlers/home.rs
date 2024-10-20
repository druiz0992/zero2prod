use actix_web::http::header::ContentType;
use actix_web::HttpResponse;

use crate::inbound::http::config::get_template_path;
use std::fs;

pub async fn home() -> HttpResponse {
    let path = get_template_path("home.html");
    let body = fs::read_to_string(path).unwrap_or_else(|_| "Failed to load login page".to_string());

    HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(body)
}
