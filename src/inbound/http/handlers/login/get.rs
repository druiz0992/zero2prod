use crate::inbound::http::{
    auth::secure_query::SecureQuery, config::get_template_path, HmacSecret,
};
use actix_web::{http::header::ContentType, web, HttpResponse};
use std::fs;

pub async fn login_form(
    query: Option<web::Query<SecureQuery>>,
    secret: web::Data<HmacSecret>,
) -> HttpResponse {
    let error_html = match query {
        None => "".into(),
        Some(query) => match query.0.verify(&secret) {
            Ok(error) => {
                format!("<p><i>{}</i></p>", htmlescape::encode_minimal(&error))
            }
            Err(e) => {
                tracing::warn!(error.message = %e, error.cause_chain = ?e,
                "Failed to verify query parameters using the HMAC tag");
                "".into()
            }
        },
    };

    let path = get_template_path("login.html");
    let html_content =
        fs::read_to_string(path).unwrap_or_else(|_| "Failed to load login page".to_string());
    let page_content = html_content.replace("{error_html}", &error_html);

    HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(page_content)
}
