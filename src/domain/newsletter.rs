const NEWSLETTER_TITLE_MAX_LENGTH: usize = 200;

pub struct Newsletter {
    pub title: NewsletterTitle,
    pub content: NewsletterContent,
}

//Click <a href=\"{}\">here</a> to confirm your subscription.",

#[derive(Debug, PartialEq)]
pub struct NewsletterTitle(String);

impl NewsletterTitle {
    fn parse(title: String) -> Result<NewsletterTitle, String> {
        if title.is_empty() || title.len() > NEWSLETTER_TITLE_MAX_LENGTH {
            Err(format!("Invalid newsletter title."))
        } else {
            Ok(Self(title))
        }
    }
}

impl AsRef<str> for NewsletterTitle {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

pub struct NewsletterContent {
    pub html: NewsletterBody,
    pub text: NewsletterBody,
}

#[derive(Debug)]
pub struct NewsletterBody(String);

impl AsRef<str> for NewsletterBody {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl NewsletterBody {
    fn parse(body: String) -> Result<NewsletterBody, String> {
        if body.is_empty() {
            return Err(format!("Newsletter body cannot be empty"));
        }
        Ok(NewsletterBody(body))
    }
}

impl Newsletter {
    pub fn parse(
        title: String,
        html_content: String,
        text_content: String,
    ) -> Result<Newsletter, String> {
        let newsletter_title = NewsletterTitle::parse(title)?;
        let newsletter_html_body = NewsletterBody::parse(html_content)?;
        let newsletter_text_body = NewsletterBody::parse(text_content)?;

        let newsletter_content = NewsletterContent {
            html: newsletter_html_body,
            text: newsletter_text_body,
        };

        Ok(Self {
            title: newsletter_title,
            content: newsletter_content,
        })
    }
}

#[cfg(test)]
use claim::assert_err;

#[test]
fn empty_newsletter_title_is_rejected() {
    let title = "".to_string();

    assert_err!(NewsletterTitle::parse(title));
}

#[test]
fn long_newsletter_title_is_rejected() {
    let title = "a".repeat(NEWSLETTER_TITLE_MAX_LENGTH + 1);

    assert_err!(NewsletterTitle::parse(title));
}

#[test]
fn correct_newsletter_title_is_accepted() {
    let title = "a".repeat(NEWSLETTER_TITLE_MAX_LENGTH);

    assert_eq!(
        NewsletterTitle::parse(title.clone()).unwrap().as_ref(),
        title
    );
}

#[test]
fn empty_newsletter_text_is_rejected() {
    let text_body = "".to_string();

    assert_err!(NewsletterBody::parse(text_body));
}

#[test]
fn correct_newsletter_text_is_accepted() {
    let text_body = "a".repeat(1000);

    assert_eq!(
        NewsletterBody::parse(text_body.clone()).unwrap().as_ref(),
        text_body
    );
}
