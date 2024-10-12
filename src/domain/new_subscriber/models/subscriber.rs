use super::email::SubscriberEmail;
use super::name::SubscriberName;

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

#[derive(Debug)]
pub struct NewSubscriber {
    pub id: SubscriberId,
    pub email: SubscriberEmail,
    pub name: SubscriberName,
    pub status: SubscriberStatus,
}

#[derive(Debug, PartialEq)]
pub enum SubscriberStatus {
    NotInserted,
    SubscriptionPendingConfirmation,
    SubscriptionConfirmed,
    CancellationPendingConfirmation,
    CancellationConfirmed,
}

impl SubscriberStatus {
    pub fn parse(status: &str) -> Result<SubscriberStatus, String> {
        match status {
            "pending_confirmation" => Ok(SubscriberStatus::SubscriptionPendingConfirmation),
            "confirmed" => Ok(SubscriberStatus::SubscriptionConfirmed),
            _ => Err("Unknown status".into()),
        }
    }
}

impl From<SubscriberStatus> for String {
    fn from(value: SubscriberStatus) -> Self {
        match value {
            SubscriberStatus::NotInserted => "not_inserted".into(),
            SubscriberStatus::SubscriptionPendingConfirmation => "pending_confirmation".into(),
            SubscriberStatus::SubscriptionConfirmed => "confirmed".into(),
            SubscriberStatus::CancellationPendingConfirmation => "cancellation_pending".into(),
            SubscriberStatus::CancellationConfirmed => "cancellation_confirmed".into(),
        }
    }
}
#[derive(thiserror::Error, Debug, PartialEq)]
pub enum NewSubscriberError {
    #[error("Invalid name: {0}")]
    InvalidName(String),
    #[error("Invalid email: {0}")]
    InvalidEmail(String),
    #[error("Duplicated email: {0}")]
    DuplicatedEmail(String),
}

impl NewSubscriber {
    pub fn new(req: NewSubscriberRequest) -> Result<NewSubscriber, NewSubscriberError> {
        Ok(Self {
            id: None,
            email: SubscriberEmail::parse(req.email).map_err(NewSubscriberError::InvalidEmail)?,
            name: SubscriberName::parse(req.name).map_err(NewSubscriberError::InvalidName)?,
            status: SubscriberStatus::NotInserted,
        })
    }
}

impl TryFrom<NewSubscriberRequest> for NewSubscriber {
    type Error = NewSubscriberError;
    fn try_from(request: NewSubscriberRequest) -> Result<Self, Self::Error> {
        NewSubscriber::new(request)
    }
}

#[cfg(test)]
mod tests {
    use super::{NewSubscriber, NewSubscriberError, NewSubscriberRequest, SubscriberStatus};

    #[test]
    fn new_subscriber_from_request_with_invalid_name_fails() {
        let email = "dada@ds.com";
        let name = "";
        let subscriber_request = NewSubscriberRequest::new(email, name);
        let subscriber = NewSubscriber::new(subscriber_request);

        assert!(matches!(
            subscriber,
            Err(NewSubscriberError::InvalidName(_))
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
            Err(NewSubscriberError::InvalidName(_))
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
            Err(NewSubscriberError::InvalidEmail(_))
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
            Err(NewSubscriberError::InvalidEmail(_))
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
