use crate::domain::new_subscriber::errors::SubscriberError;
use crate::domain::newsletter::errors::NewsletterError;
use validator::validate_email;

#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct SubscriberEmail(String);
impl SubscriberEmail {
    pub fn parse(s: String) -> Result<SubscriberEmail, EmailError> {
        if validate_email(&s) {
            Ok(Self(s))
        } else {
            Err(EmailError::InvalidSubscriber(format!(
                "{} is not a valid email",
                s
            )))
        }
    }
}
impl AsRef<str> for SubscriberEmail {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for SubscriberEmail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl From<SubscriberEmail> for String {
    fn from(email: SubscriberEmail) -> Self {
        email.0
    }
}

#[derive(thiserror::Error, Debug)]
pub enum EmailError {
    #[error("Invalid email subject {0}")]
    InvalidSubject(String),
    #[error("Invalid email Html content: {0}")]
    InvalidHtmlContent(String),
    #[error("Invalid email text content: {0}")]
    InvalidTextContent(String),
    #[error("Invalid subscriber email: {0}")]
    InvalidSubscriber(String),
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct EmailMessage {
    subject: EmailSubject,
    html_content: EmailHtmlContent,
    text_content: EmailTextContent,
}

impl EmailMessage {
    pub fn new(
        subject: EmailSubject,
        html_content: EmailHtmlContent,
        text_content: EmailTextContent,
    ) -> Self {
        Self {
            subject,
            html_content,
            text_content,
        }
    }
    pub fn subject_as_ref(&self) -> &EmailSubject {
        &self.subject
    }
    pub fn html_as_ref(&self) -> &EmailHtmlContent {
        &self.html_content
    }
    pub fn text_as_ref(&self) -> &EmailTextContent {
        &self.text_content
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct EmailSubject(String);

impl TryFrom<String> for EmailSubject {
    type Error = EmailError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        EmailSubject::try_from(value.as_str())
    }
}

impl TryFrom<&str> for EmailSubject {
    type Error = EmailError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if !value.is_empty() {
            Ok(Self(value.to_string()))
        } else {
            Err(EmailError::InvalidSubject(
                "EmailSubject cannot be empty.".into(),
            ))
        }
    }
}
impl AsRef<str> for EmailSubject {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct EmailHtmlContent(String);

impl TryFrom<String> for EmailHtmlContent {
    type Error = EmailError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        EmailHtmlContent::try_from(value.as_str())
    }
}

impl TryFrom<&str> for EmailHtmlContent {
    type Error = EmailError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if !value.is_empty() {
            Ok(Self(value.to_string()))
        } else {
            Err(EmailError::InvalidHtmlContent(
                "EmailHtmlContent cannot be empty.".into(),
            ))
        }
    }
}
impl AsRef<str> for EmailHtmlContent {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct EmailTextContent(String);

impl TryFrom<String> for EmailTextContent {
    type Error = EmailError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        EmailTextContent::try_from(value.as_str())
    }
}

impl TryFrom<&str> for EmailTextContent {
    type Error = EmailError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if !value.is_empty() {
            Ok(Self(value.to_string()))
        } else {
            Err(EmailError::InvalidTextContent(
                "EmailTextContent cannot be empty.".into(),
            ))
        }
    }
}
impl AsRef<str> for EmailTextContent {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::{
        EmailError, EmailHtmlContent, EmailMessage, EmailSubject, EmailTextContent, SubscriberEmail,
    };
    use claim::assert_err;
    use fake::faker::internet::en::SafeEmail;
    use fake::Fake;

    #[derive(Debug, Clone)]
    struct ValidEmailFixture(pub String);

    impl quickcheck::Arbitrary for ValidEmailFixture {
        fn arbitrary<G: quickcheck::Gen>(g: &mut G) -> Self {
            let email = SafeEmail().fake_with_rng(g);
            Self(email)
        }
    }

    #[quickcheck_macros::quickcheck]
    fn valid_subscriber_emails_are_parsed_successfully(valid_email: ValidEmailFixture) -> bool {
        SubscriberEmail::parse(valid_email.0).is_ok()
    }

    #[test]
    fn empty_subscriber_email_string_is_rejected() {
        let email = "".to_string();
        assert_err!(SubscriberEmail::parse(email));
    }

    #[test]
    fn subsciber_email_missing_at_symbol_is_rejected() {
        let email = "ursuladomain.com".to_string();
        assert_err!(SubscriberEmail::parse(email));
    }

    #[test]
    fn subscriber_email_missing_subject_is_rejected() {
        let email = "@domain.com".to_string();
        assert_err!(SubscriberEmail::parse(email));
    }

    #[test]
    fn subscriber_email_with_two_letter_top_level_domain_is_accepted() {
        let email = "hello@domain.ai".to_string();
        assert_eq!(
            SubscriberEmail::parse(email.clone()).unwrap().as_ref(),
            email
        )
    }

    #[test]
    fn email_message_with_empty_subject_is_rejected() {
        let email_subject: Result<EmailSubject, EmailError> = "".try_into();
        if let Err(EmailError::InvalidSubject(msg)) = email_subject {
            assert_eq!(
                msg, "EmailSubject cannot be empty.",
                "Error message does not match"
            );
        } else {
            panic!("Expected EmailError::InvalidSubject, got something else");
        }
    }
    #[test]
    fn email_message_with_empty_html_content_is_rejected() {
        let html_content: Result<EmailHtmlContent, EmailError> = "".try_into();
        if let Err(EmailError::InvalidHtmlContent(msg)) = html_content {
            assert_eq!(
                msg, "EmailHtmlContent cannot be empty.",
                "Error message does not match"
            );
        } else {
            panic!("Expected EmailError::InvalidHtmlContent, got something else");
        }
    }

    #[test]
    fn email_message_with_empty_text_content_is_rejected() {
        let text_content: Result<EmailTextContent, EmailError> = "".try_into();
        if let Err(EmailError::InvalidTextContent(msg)) = text_content {
            assert_eq!(
                msg, "EmailTextContent cannot be empty.",
                "Error message does not match"
            );
        } else {
            panic!("Expected EmailError::InvalidTextContent, got something else");
        }
    }
    #[test]
    fn valid_email_message_is_accepted() {
        let subject: EmailSubject = "welcome".try_into().unwrap();
        let html_content: EmailHtmlContent = "fffffff".try_into().unwrap();
        let text_content: EmailTextContent = "fffffff".try_into().unwrap();

        let _ = EmailMessage {
            subject,
            html_content,
            text_content,
        };
    }
}
