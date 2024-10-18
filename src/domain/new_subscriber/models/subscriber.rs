use super::{
    email::{EmailError, SubscriberEmail},
    name::{SubscriberName, SubscriberNameError},
};
use crate::domain::new_subscriber::errors::SubscriberError;

#[derive(serde::Deserialize)]
pub struct NewSubscriberRequest {
    pub email: String,
    pub name: String,
}

impl NewSubscriberRequest {
    pub fn new(email: &str, name: &str) -> NewSubscriberRequest {
        Self {
            email: email.to_string(),
            name: name.to_string(),
        }
    }
}

pub type SubscriberId = Option<uuid::Uuid>;

#[derive(Debug, Clone)]
pub struct NewSubscriber {
    pub id: SubscriberId,
    pub email: SubscriberEmail,
    pub name: SubscriberName,
    pub status: SubscriberStatus,
}

#[derive(thiserror::Error, Debug)]
pub enum SubscriberStatusError {
    #[error("Unknown subscriber status: {0}")]
    UnknownStatus(String),
}

#[derive(Debug, PartialEq, Clone)]
pub enum SubscriberStatus {
    NotInserted,
    SubscriptionPendingConfirmation,
    SubscriptionConfirmed,
    CancellationPendingConfirmation,
    CancellationConfirmed,
}

impl SubscriberStatus {
    const SUBSCRIPTION_PENDING_CONFIRMATION: &'static str = "pending_confirmation";
    const SUBSCRIPTION_CONFIRMED: &'static str = "confirmed";
    const SUBSCRIBER_NOT_INSERTED: &'static str = "not_inserted";
    const CANCELLATION_PENDING_CONFIRMATION: &'static str = "cancellation_pending";
    const CANCELLATION_CONFIRMED: &'static str = "cancellation_confirmed";

    pub fn parse(status: &str) -> Result<SubscriberStatus, SubscriberStatusError> {
        match status {
            Self::SUBSCRIPTION_PENDING_CONFIRMATION => {
                Ok(SubscriberStatus::SubscriptionPendingConfirmation)
            }
            Self::SUBSCRIPTION_CONFIRMED => Ok(SubscriberStatus::SubscriptionConfirmed),
            Self::SUBSCRIBER_NOT_INSERTED => Ok(SubscriberStatus::NotInserted),
            Self::CANCELLATION_PENDING_CONFIRMATION => {
                Ok(SubscriberStatus::CancellationPendingConfirmation)
            }
            Self::CANCELLATION_CONFIRMED => Ok(SubscriberStatus::CancellationConfirmed),
            _ => Err(SubscriberStatusError::UnknownStatus(status.into())),
        }
    }
}

impl From<SubscriberStatusError> for SubscriberError {
    fn from(error: SubscriberStatusError) -> Self {
        Self::ValidationError(format!("Invalid status {}", error.to_string()))
    }
}
impl From<SubscriberStatus> for String {
    fn from(value: SubscriberStatus) -> Self {
        match value {
            SubscriberStatus::NotInserted => SubscriberStatus::SUBSCRIBER_NOT_INSERTED.into(),
            SubscriberStatus::SubscriptionPendingConfirmation => {
                SubscriberStatus::SUBSCRIPTION_PENDING_CONFIRMATION.into()
            }
            SubscriberStatus::SubscriptionConfirmed => {
                SubscriberStatus::SUBSCRIPTION_CONFIRMED.into()
            }
            SubscriberStatus::CancellationPendingConfirmation => {
                SubscriberStatus::CANCELLATION_PENDING_CONFIRMATION.into()
            }
            SubscriberStatus::CancellationConfirmed => {
                SubscriberStatus::CANCELLATION_CONFIRMED.into()
            }
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum SubscriberValidationError {
    #[error("Invalid subscriber name: {0}")]
    InvalidName(#[from] SubscriberNameError),
    #[error("Invalid subscriber email: {0}")]
    InvalidEmail(#[from] EmailError),
}

impl NewSubscriber {
    pub fn new(req: NewSubscriberRequest) -> Result<NewSubscriber, SubscriberValidationError> {
        Ok(Self {
            id: None,
            email: SubscriberEmail::parse(req.email)
                .map_err(SubscriberValidationError::InvalidEmail)?,
            name: SubscriberName::parse(req.name)
                .map_err(SubscriberValidationError::InvalidName)?,
            status: SubscriberStatus::NotInserted,
        })
    }

    pub fn build(name: SubscriberName, email: SubscriberEmail) -> NewSubscriber {
        Self {
            id: None,
            status: SubscriberStatus::NotInserted,
            name,
            email,
        }
    }

    pub fn with_id(self, id: SubscriberId) -> Self {
        Self { id, ..self }
    }

    pub fn with_status(self, status: SubscriberStatus) -> Self {
        Self { status, ..self }
    }
}

impl TryFrom<NewSubscriberRequest> for NewSubscriber {
    type Error = SubscriberValidationError;
    fn try_from(request: NewSubscriberRequest) -> Result<Self, Self::Error> {
        NewSubscriber::new(request)
    }
}

impl From<SubscriberValidationError> for SubscriberError {
    fn from(error: SubscriberValidationError) -> Self {
        Self::ValidationError(error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::{NewSubscriber, NewSubscriberRequest, SubscriberStatus, SubscriberValidationError};

    #[test]
    fn new_subscriber_from_request_with_invalid_name_fails() {
        let email = "dada@ds.com";
        let name = "";
        let subscriber_request = NewSubscriberRequest::new(email, name);
        let subscriber = NewSubscriber::new(subscriber_request);

        assert!(matches!(
            subscriber,
            Err(SubscriberValidationError::InvalidName(_))
        ));
    }
    #[test]
    fn subscriber_try_from_request_with_invalid_name_fails() {
        let email = "dada@ds.com";
        let name = "";
        let subscriber_request = NewSubscriberRequest::new(email, name);
        let subscriber = NewSubscriber::try_from(subscriber_request);

        assert!(matches!(
            subscriber,
            Err(SubscriberValidationError::InvalidName(_))
        ));
    }

    #[test]
    fn new_subscriber_request_with_invalid_email_fails() {
        let email = "";
        let name = "dada";
        let subscriber_request = NewSubscriberRequest::new(email, name);
        let subscriber = NewSubscriber::new(subscriber_request);

        assert!(matches!(
            subscriber,
            Err(SubscriberValidationError::InvalidEmail(_))
        ));
    }

    #[test]
    fn subscriber_try_from_request_with_invalid_email_fails() {
        let email = "";
        let name = "dada";
        let subscriber_request = NewSubscriberRequest::new(email, name);
        let subscriber = NewSubscriber::try_from(subscriber_request);

        assert!(matches!(
            subscriber,
            Err(SubscriberValidationError::InvalidEmail(_))
        ));
    }

    #[test]
    fn new_subscriber_request_can_be_converted_into_new_subscriber() {
        let email = "dada@ds.com";
        let name = "dada";
        let subscriber_request = NewSubscriberRequest::new(email, name);
        let subscriber = NewSubscriber::new(subscriber_request).unwrap();

        assert_eq!(subscriber.email.as_ref(), email);
        assert_eq!(subscriber.name.as_ref(), name);
        assert_eq!(subscriber.status, SubscriberStatus::NotInserted,);
        assert!(matches!(subscriber.id, None));
    }

    #[test]
    fn subscriber_try_from_request_can_be_converted_into_new_subscriber() {
        let email = "dada@ds.com";
        let name = "dada";
        let subscriber_request = NewSubscriberRequest::new(email, name);
        let subscriber = NewSubscriber::try_from(subscriber_request).unwrap();

        assert_eq!(subscriber.email.as_ref(), email);
        assert_eq!(subscriber.name.as_ref(), name);
        assert_eq!(subscriber.status, SubscriberStatus::NotInserted,);
        assert!(matches!(subscriber.id, None));
    }
}
