use async_trait::async_trait;

use super::*;
use crate::domain::new_subscriber::errors::SubscriberError;

impl EmailClient {
    fn build_subscriber_notification(
        &self,
        subscription_token: SubscriptionToken,
    ) -> Result<EmailMessage, SubscriberError> {
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
}

#[async_trait]
impl SubscriptionNotifier for EmailClient {
    #[tracing::instrument(
        name = "Send a confirmation email to a new subscriber",
        skip(self, recipient, token)
    )]
    async fn send_subscriber_notification(
        &self,
        recipient: &SubscriberEmail,
        token: SubscriptionToken,
    ) -> Result<(), SubscriberError> {
        let message = self.build_subscriber_notification(token)?;
        let subject = message.subject_as_ref();
        let html_content = message.html_as_ref();
        let text_content = message.text_as_ref();
        let request_body = SendEmailRequest {
            from: self.sender.as_ref(),
            to: recipient.as_ref(),
            subject: subject.as_ref(),
            html_body: html_content.as_ref(),
            text_body: text_content.as_ref(),
        };
        self.send_notification(request_body)
            .await
            .map_err(SubscriberError::Unexpected)
    }
}

#[cfg(test)]
mod tests {
    use crate::configuration::EmailClientSettings;
    use crate::domain::new_subscriber::models::email::SubscriberEmail;
    use crate::domain::new_subscriber::models::token::SubscriptionToken;
    use crate::domain::new_subscriber::ports::SubscriptionNotifier;
    use crate::outbound::notifier::email_client::EmailClient;
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

        let _ = email_client
            .send_subscriber_notification(&email(), subscription_token)
            .await;
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

        let outcome = email_client
            .send_subscriber_notification(&email(), subscription_token)
            .await;

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

        let outcome = email_client
            .send_subscriber_notification(&email(), subscription_token)
            .await;

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

        let outcome = email_client
            .send_subscriber_notification(&email(), subscription_token)
            .await;

        assert_err!(outcome);
    }
}
