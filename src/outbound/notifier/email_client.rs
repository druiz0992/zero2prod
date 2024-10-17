use crate::configuration::EmailClientSettings;
use crate::domain::new_subscriber::{
    models::{
        email::{EmailHtmlContent, EmailMessage, EmailSubject, EmailTextContent, SubscriberEmail},
        token::SubscriptionToken,
    },
    ports::SubscriptionNotifier,
};
use crate::domain::newsletter::models::newsletter::Newsletter;
use crate::domain::newsletter::ports::NewsletterNotifier;
use reqwest::Client;
use secrecy::{ExposeSecret, Secret};

mod newsletter_notifier;
mod subscriber_notifier;

#[derive(Debug, Clone)]
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

#[derive(serde::Serialize)]
#[serde(rename_all = "PascalCase")]
struct SendEmailRequest<'a> {
    from: &'a str,
    to: &'a str,
    subject: &'a str,
    html_body: &'a str,
    text_body: &'a str,
}
