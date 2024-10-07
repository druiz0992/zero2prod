use crate::routes::error_chain_fmt;
use actix_web::{http::StatusCode, ResponseError};

#[derive(thiserror::Error)]
pub enum ConfirmationError {
    #[error("{0}")]
    ValidationError(String),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
    #[error("There is no subscriber associated with provided token")]
    UnknownToken,
}

impl std::fmt::Debug for ConfirmationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for ConfirmationError {
    fn status_code(&self) -> StatusCode {
        match self {
            ConfirmationError::ValidationError(_) => StatusCode::BAD_REQUEST,
            ConfirmationError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ConfirmationError::UnknownToken => StatusCode::UNAUTHORIZED,
        }
    }
}
