use crate::domain::new_subscriber::models::{email::EmailError, name::SubscriberNameError};

#[derive(thiserror::Error, Debug)]
pub enum SubscriberError {
    #[error("Validation error: {0}")]
    ValidationError(String),
    #[error("Subscriber not found: {0}")]
    NotFound(String),
    #[error("Subscriber not authenticated: {0}")]
    AuthError(String),
    #[error(transparent)]
    Unexpected(#[from] anyhow::Error),
}

impl From<EmailError> for SubscriberError {
    fn from(value: EmailError) -> Self {
        Self::ValidationError(value.to_string())
    }
}

impl From<SubscriberNameError> for SubscriberError {
    fn from(value: SubscriberNameError) -> Self {
        Self::ValidationError(value.to_string())
    }
}
