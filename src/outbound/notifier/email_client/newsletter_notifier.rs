use crate::domain::newsletter::errors::NewsletterError;
use crate::domain::newsletter::models::newsletter::{
    NewsletterBody, NewsletterBodyWrapper, NewsletterHtmlBody, NewsletterTextBody,
};
use async_trait::async_trait;

use super::*;

impl EmailClient {
    fn build_newsletter(
        &self,
        subscription_token: SubscriptionToken,
    ) -> Result<EmailMessage, NewsletterError> {
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
impl NewsletterNotifier for EmailClient {
    #[tracing::instrument(
        name = "Send newsletter to confirmed subscriber",
        skip(self, recipient, token, newsletter)
    )]
    async fn send_newsletter(
        &self,
        recipient: &SubscriberEmail,
        newsletter: &Newsletter,
        token: SubscriptionToken,
        base_url: &str,
    ) -> Result<(), NewsletterError> {
        let unsubscribe_link = build_unsubscribe_link(&base_url, &token);
        let html_content = embed_link_to_html_content(&newsletter.content.html, &unsubscribe_link);
        let text_content = embed_link_to_text_content(&newsletter.content.text, &unsubscribe_link);
        let subject = newsletter.title.as_ref();
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
            .map_err(|e| NewsletterError::Unexpected(anyhow::Error::from(e)))?
            .error_for_status()
            .map_err(|e| NewsletterError::Unexpected(anyhow::Error::from(e)))?;

        Ok(())
    }
}

fn embed_link_to_text_content(
    body: &NewsletterBodyWrapper<NewsletterTextBody>,
    link: &str,
) -> String {
    let text_with_link = format!(
        "\nClick <a href=\"{}\">here</a> to unsubscribe from newsletter.",
        link
    );
    let content_with_link = format!("{} {} ", body.as_ref(), text_with_link);
    content_with_link
}
fn embed_link_to_html_content(
    body: &NewsletterBodyWrapper<NewsletterHtmlBody>,
    link: &str,
) -> String {
    let text_with_link = format!("\nClick here {} to unsubscribe from newsletter.", link);
    let content_with_link = format!("{} {} ", body.as_ref(), text_with_link);
    content_with_link
}

fn build_unsubscribe_link(base_url: &str, token: &SubscriptionToken) -> String {
    let unsubscribe_link = format!(
        "{}/subscriptions/unsubscribe?token={}",
        base_url,
        token.as_ref()
    );
    unsubscribe_link
}
