use crate::domain::auth::credentials::CredentialsError;
use crate::domain::new_subscriber::errors::SubscriberError;
use crate::domain::newsletter::errors::NewsletterError;

use actix_web::http::header::{self, HeaderValue};
use actix_web::HttpResponse;
use actix_web::{http::StatusCode, ResponseError};

#[derive(thiserror::Error, Debug)]
pub enum AppError {
    #[error("Validation error: {0}")]
    ValidationError(String),
    #[error("Subscriber not found: {0}")]
    NotFound(String),
    #[error("Subscriber not authenticated: {0}")]
    AuthError(String),
    #[error(transparent)]
    Unexpected(#[from] anyhow::Error),
}

impl From<SubscriberError> for AppError {
    fn from(error: SubscriberError) -> Self {
        match error {
            SubscriberError::ValidationError(s) => AppError::ValidationError(s),
            SubscriberError::AuthError(s) => AppError::AuthError(s),
            SubscriberError::NotFound(s) => AppError::NotFound(s),
            SubscriberError::Unexpected(s) => AppError::Unexpected(s),
        }
    }
}

impl From<NewsletterError> for AppError {
    fn from(error: NewsletterError) -> Self {
        match error {
            NewsletterError::ValidationError(s) => AppError::ValidationError(s),
            NewsletterError::NotFound(s) => AppError::NotFound(s),
            NewsletterError::Unexpected(s) => AppError::Unexpected(s),
            NewsletterError::AuthError(s) => AppError::AuthError(s),
        }
    }
}
impl From<CredentialsError> for AppError {
    fn from(error: CredentialsError) -> Self {
        match error {
            CredentialsError::Unexpected(s) => AppError::Unexpected(s),
            CredentialsError::AuthError(s) => AppError::AuthError(s),
        }
    }
}

impl ResponseError for AppError {
    fn status_code(&self) -> StatusCode {
        match self {
            AppError::ValidationError(_) => StatusCode::BAD_REQUEST,
            AppError::Unexpected(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::AuthError(_) => StatusCode::UNAUTHORIZED,
        }
    }

    fn error_response(&self) -> actix_web::HttpResponse<actix_web::body::BoxBody> {
        match self {
            AppError::ValidationError(_) => HttpResponse::new(StatusCode::BAD_REQUEST),
            AppError::Unexpected(_) => HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR),
            AppError::NotFound(_) => HttpResponse::new(StatusCode::NOT_FOUND),
            AppError::AuthError(_) => {
                let mut response = HttpResponse::new(StatusCode::UNAUTHORIZED);
                let header_value = HeaderValue::from_str(r#"Basic realm="publish""#).unwrap();
                response
                    .headers_mut()
                    .insert(header::WWW_AUTHENTICATE, header_value);
                response
            }
        }
    }
}
