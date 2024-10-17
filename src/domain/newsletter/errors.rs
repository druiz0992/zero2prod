use crate::domain::new_subscriber::errors::SubscriberError;
use crate::domain::new_subscriber::models::{email::EmailError, name::SubscriberNameError};

#[derive(thiserror::Error, Debug)]
pub enum NewsletterError {
    #[error("Validation error: {0}")]
    ValidationError(String),
    #[error("Subscriber not found: {0}")]
    NotFound(String),
    #[error("Subscriber not authenticated: {0}")]
    AuthError(String),
    #[error(transparent)]
    Unexpected(#[from] anyhow::Error),
}

impl From<EmailError> for NewsletterError {
    fn from(value: EmailError) -> Self {
        Self::ValidationError(value.to_string())
    }
}

impl From<SubscriberNameError> for NewsletterError {
    fn from(value: SubscriberNameError) -> Self {
        Self::ValidationError(value.to_string())
    }
}

impl From<SubscriberError> for NewsletterError {
    fn from(error: SubscriberError) -> Self {
        match error {
            SubscriberError::AuthError(e) => NewsletterError::AuthError(e),
            SubscriberError::NotFound(e) => NewsletterError::NotFound(e),
            SubscriberError::Unexpected(e) => NewsletterError::Unexpected(e),
            SubscriberError::ValidationError(e) => NewsletterError::ValidationError(e),
        }
    }
}
