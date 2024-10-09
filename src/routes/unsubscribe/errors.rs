use crate::routes::error_chain_fmt;
use actix_web::{http::StatusCode, ResponseError};

#[derive(thiserror::Error)]
pub enum UnsubscribeError {
    #[error("{0}")]
    ValidationError(String),
}

impl std::fmt::Debug for UnsubscribeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for UnsubscribeError {
    fn status_code(&self) -> StatusCode {
        match self {
            UnsubscribeError::ValidationError(_) => StatusCode::BAD_REQUEST,
        }
    }
}
