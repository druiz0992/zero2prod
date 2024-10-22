use actix_web::http::header::ContentType;
use actix_web::HttpResponse;

use crate::inbound::http::utils::{load_html, HtmlTemplate};

pub async fn home() -> HttpResponse {
    let body = load_html(HtmlTemplate::Home);

    HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(body)
}
