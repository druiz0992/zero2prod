use async_trait::async_trait;

use super::models::{
    email::{EmailError, EmailMessage, SubscriberEmail},
    subscriber::{NewSubscriber, NewSubscriberError, NewSubscriberRequest, SubscriberStatusError},
    token::{SubscriptionToken, SubscriptionTokenError, SubscriptionTokenRequest},
};

#[async_trait]
///  Represents a store of subscriber data
pub trait SubscriberRepository: Send + Sync + 'static {
    /// Asynchronously retrieves a subscriber and token if it exists,
    ///  or creates a new entry for the provided `NewSubscriberRequest`
    async fn retrieve_or_insert(
        &self,
        subscriber: NewSubscriberRequest,
        token: SubscriptionToken,
    ) -> Result<(NewSubscriber, SubscriptionToken), SubscriberRepositoryError>;

    /// Asynchronously updates a subscriber in repository
    async fn update(&self, subscriber: NewSubscriber) -> Result<(), SubscriberRepositoryError>;

    /// Asynchronously retrieve a subscriber from a token
    async fn retrieve_from_token(
        &self,
        token: &SubscriptionToken,
    ) -> Result<NewSubscriber, SubscriberRepositoryError>;
}

#[derive(thiserror::Error, Debug)]
pub enum SubscriberRepositoryError {
    #[error("Subscriber validation error: {0}")]
    InvalidSubscriberValidation(#[from] NewSubscriberError),

    #[error("Subscriber status error: {0}")]
    InvalidSubscriberStatus(#[from] SubscriberStatusError),

    #[error("Token validation error: {0}")]
    InvalidSubscriptionToken(#[from] SubscriptionTokenError),

    #[error("Subscriber not found")]
    SubscriberNotFound,

    #[error("There is no subscriber associated with provided token")]
    UnknownToken,

    #[error(transparent)]
    Unexpected(#[from] anyhow::Error),
}

#[async_trait]
pub trait SubscriptionService: Send + Sync + 'static {
    async fn new_subscriber(
        &self,
        req: NewSubscriberRequest,
    ) -> Result<NewSubscriber, SubscriptionServiceError>;

    async fn confirm(
        &self,
        req: SubscriptionTokenRequest,
    ) -> Result<NewSubscriber, SubscriptionServiceError>;
}

#[derive(thiserror::Error, Debug)]
pub enum SubscriptionServiceError {
    #[error("Error in repository: {0}")]
    RepositoryValidationError(SubscriberRepositoryError),

    #[error("Error in notifier: {0}")]
    NotifierValidationError(SubscriptionNotifierError),

    #[error("Error in subscription token: {0}")]
    TokenValidationError(#[from] SubscriptionTokenError),

    #[error("Subscriber not found in repo")]
    RepositorySubscriberNotFound,

    #[error("There is no subscriber associated with provided token")]
    UnknownToken,

    #[error(transparent)]
    Unexpected(#[from] anyhow::Error),
}

impl From<SubscriberRepositoryError> for SubscriptionServiceError {
    fn from(error: SubscriberRepositoryError) -> Self {
        match error {
            SubscriberRepositoryError::Unexpected(e) => SubscriptionServiceError::Unexpected(e),
            SubscriberRepositoryError::SubscriberNotFound => {
                SubscriptionServiceError::RepositorySubscriberNotFound
            }
            SubscriberRepositoryError::UnknownToken => SubscriptionServiceError::UnknownToken,
            _ => SubscriptionServiceError::RepositoryValidationError(error),
        }
    }
}

impl From<SubscriptionNotifierError> for SubscriptionServiceError {
    fn from(error: SubscriptionNotifierError) -> Self {
        match error {
            SubscriptionNotifierError::Unexpected(e) => SubscriptionServiceError::Unexpected(e),
            _ => SubscriptionServiceError::NotifierValidationError(error),
        }
    }
}

#[async_trait]
pub trait SubscriptionNotifier: Send + Sync + 'static {
    fn build_notification(
        &self,
        subscription_token: SubscriptionToken,
    ) -> Result<EmailMessage, SubscriptionNotifierError>;

    async fn send_notification(
        &self,
        recipient: &SubscriberEmail,
        message: &EmailMessage,
    ) -> Result<(), SubscriptionNotifierError>;
}

#[derive(thiserror::Error, Debug)]
pub enum SubscriptionNotifierError {
    #[error("Validation error: {0}")]
    InvalidEmailMessage(#[from] EmailError),

    #[error(transparent)]
    Unexpected(#[from] anyhow::Error),
}
