use crate::configuration::EmailClientSettings;
use crate::domain::new_subscriber::{
    models::{
        email::{EmailHtmlContent, EmailMessage, EmailSubject, EmailTextContent, SubscriberEmail},
        token::SubscriptionToken,
    },
    ports::{SubscriptionNotifier, SubscriptionNotifierError},
};
use async_trait::async_trait;
use reqwest::Client;
use secrecy::{ExposeSecret, Secret};

#[derive(Debug)]
pub struct EmailClient {
    http_client: Client,
    base_url: String,
    sender: SubscriberEmail,
    authorization_token: Secret<String>,
}

impl EmailClient {
    pub fn new(configuration: EmailClientSettings) -> Self {
        let sender = configuration
            .sender()
            .expect("Invalid sender email address");
        let timeout = configuration.timeout();

        let http_client = Client::builder().timeout(timeout).build().unwrap();
        Self {
            http_client,
            base_url: configuration.base_url,
            sender,
            authorization_token: configuration.authorization_token,
        }
    }
}

#[async_trait]
impl SubscriptionNotifier for EmailClient {
    fn build_notification(
        &self,
        subscription_token: SubscriptionToken,
    ) -> Result<EmailMessage, SubscriptionNotifierError> {
        let confirmation_link = format!(
            "{}/subscriptions/confirm?subscription_token={}",
            self.base_url,
            subscription_token.as_ref()
        );
        let text_content = EmailTextContent::try_from(format!(
            "Welcome to our newsletter!<br />\
            Click <a href=\"{}\">here</a> to confirm your subscription.",
            confirmation_link
        ))?;

        let html_content = EmailHtmlContent::try_from(format!(
            "Welcome to our newsletter!\nClick here {} to confirm your subscription.",
            confirmation_link
        ))?;

        let subject = EmailSubject::try_from("Welcome")?;

        Ok(EmailMessage::new(subject, html_content, text_content))
    }

    #[tracing::instrument(
        name = "Send a confirmation email to a new subscriber",
        skip(self, recipient, message)
    )]
    async fn send_notification(
        &self,
        recipient: &SubscriberEmail,
        message: &EmailMessage,
    ) -> Result<(), SubscriptionNotifierError> {
        let subject = message.subject_as_ref();
        let html_content = message.html_as_ref();
        let text_content = message.text_as_ref();
        let url = format!("{}/email", self.base_url);
        let request_body = SendEmailRequest {
            from: self.sender.as_ref(),
            to: recipient.as_ref(),
            subject: subject.as_ref(),
            html_body: html_content.as_ref(),
            text_body: text_content.as_ref(),
        };
        let _builder = self
            .http_client
            .post(&url)
            .header(
                "X-Postmark-Server-Token",
                self.authorization_token.expose_secret(),
            )
            .json(&request_body)
            .send()
            .await
            .map_err(|e| SubscriptionNotifierError::Unexpected(anyhow::Error::from(e)))?
            .error_for_status()
            .map_err(|e| SubscriptionNotifierError::Unexpected(anyhow::Error::from(e)))?;

        Ok(())
    }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "PascalCase")]
struct SendEmailRequest<'a> {
    from: &'a str,
    to: &'a str,
    subject: &'a str,
    html_body: &'a str,
    text_body: &'a str,
}

#[cfg(test)]
mod tests {
    use crate::configuration::EmailClientSettings;
    use crate::domain::new_subscriber::models::email::SubscriberEmail;
    use crate::domain::new_subscriber::models::token::SubscriptionToken;
    use crate::domain::new_subscriber::ports::SubscriptionNotifier;
    use crate::outbound::subscription_notifier::email_client::EmailClient;
    use claim::{assert_err, assert_ok};
    use fake::faker::internet::en::SafeEmail;
    use fake::{Fake, Faker};
    use secrecy::Secret;
    use wiremock::matchers::{any, header, header_exists, method, path};
    use wiremock::{Mock, MockServer, Request, ResponseTemplate};

    fn email() -> SubscriberEmail {
        SubscriberEmail::parse(SafeEmail().fake()).unwrap()
    }

    fn email_client(base_url: String) -> EmailClient {
        let configuration = EmailClientSettings {
            base_url,
            sender_email: email().into(),
            authorization_token: Secret::new(Faker.fake()),
            timeout_milliseconds: 200,
        };
        EmailClient::new(configuration)
    }

    struct SendEmailBodyMatcher;

    impl wiremock::Match for SendEmailBodyMatcher {
        fn matches(&self, request: &Request) -> bool {
            let result: Result<serde_json::Value, _> = serde_json::from_slice(&request.body);
            if let Ok(body) = result {
                body.get("From").is_some()
                    && body.get("To").is_some()
                    && body.get("HtmlBody").is_some()
                    && body.get("TextBody").is_some()
            } else {
                false
            }
        }
    }

    #[tokio::test]
    async fn send_email_sends_the_expected_request() {
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri());

        Mock::given(header_exists("X-Postmark-Server-Token"))
            .and(header("Content-Type", "application/json"))
            .and(path("/email"))
            .and(method("POST"))
            .and(SendEmailBodyMatcher)
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        let subscription_token = SubscriptionToken::default();

        let message = email_client.build_notification(subscription_token).unwrap();
        let _ = email_client.send_notification(&email(), &message).await;
    }

    #[tokio::test]
    async fn send_email_succeeds_if_the_server_returns_200() {
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri());

        Mock::given(any())
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        let subscription_token = SubscriptionToken::default();

        let message = email_client.build_notification(subscription_token).unwrap();
        let outcome = email_client.send_notification(&email(), &message).await;

        assert_ok!(outcome);
    }

    #[tokio::test]
    async fn send_email_fails_if_the_server_returns_500() {
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri());

        Mock::given(any())
            .respond_with(ResponseTemplate::new(500))
            .expect(1)
            .mount(&mock_server)
            .await;

        let subscription_token = SubscriptionToken::default();

        let message = email_client.build_notification(subscription_token).unwrap();
        let outcome = email_client.send_notification(&email(), &message).await;

        assert_err!(outcome);
    }

    #[tokio::test]
    async fn send_email_times_out_if_the_server_takes_too_long() {
        // Arrange
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri());

        let response = ResponseTemplate::new(200).set_delay(std::time::Duration::from_secs(180));
        Mock::given(any())
            .respond_with(response)
            .expect(1)
            .mount(&mock_server)
            .await;

        let subscription_token = SubscriptionToken::default();

        let message = email_client.build_notification(subscription_token).unwrap();
        let outcome = email_client.send_notification(&email(), &message).await;

        assert_err!(outcome);
    }
}
