use crate::domain::newsletter::errors::NewsletterError;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct NewsletterDto {
    pub title: String,
    pub content: NewsletterContentDto,
}

#[derive(Deserialize, Debug)]
pub struct NewsletterContentDto {
    pub html: String,
    pub text: String,
}

pub struct Newsletter {
    pub title: NewsletterTitle,
    pub content: NewsletterContent,
}

#[derive(Debug, PartialEq)]
pub struct NewsletterTitle(String);

impl NewsletterTitle {
    const MAX_LENGTH: usize = 200;

    pub fn parse(title: String) -> Result<NewsletterTitle, String> {
        if title.is_empty() || title.len() > NewsletterTitle::MAX_LENGTH {
            Err("Invalid newsletter title.".to_string())
        } else {
            Ok(Self(title))
        }
    }
    pub fn as_ref(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, PartialEq)]
pub struct NewsletterContent {
    pub html: NewsletterBodyWrapper<NewsletterHtmlBody>,
    pub text: NewsletterBodyWrapper<NewsletterTextBody>,
}

#[derive(Debug, PartialEq)]
pub struct NewsletterBodyWrapper<B> {
    body: NewsletterBody,
    _marker: std::marker::PhantomData<B>,
}

#[derive(Debug, PartialEq)]
pub struct NewsletterHtmlBody;

#[derive(Debug, PartialEq)]
pub struct NewsletterTextBody;

#[derive(Debug, PartialEq)]
pub struct NewsletterBody(String);

impl NewsletterBody {
    pub fn as_ref(&self) -> &str {
        &self.0
    }
}

impl<B> NewsletterBodyWrapper<B> {
    pub fn new(body: String) -> Result<Self, String> {
        if body.is_empty() {
            return Err("Newsletter body cannot be empty".to_string());
        }
        Ok(Self {
            body: NewsletterBody(body),
            _marker: std::marker::PhantomData,
        })
    }

    pub fn as_ref(&self) -> &str {
        &self.body.0
    }
}

impl TryFrom<NewsletterDto> for Newsletter {
    type Error = NewsletterError;

    fn try_from(dto: NewsletterDto) -> Result<Self, Self::Error> {
        let newsletter_title =
            NewsletterTitle::parse(dto.title).map_err(|e| NewsletterError::ValidationError(e))?;
        let newsletter_html_body =
            NewsletterBodyWrapper::<NewsletterHtmlBody>::new(dto.content.html)
                .map_err(|e| NewsletterError::ValidationError(e))?;
        let newsletter_text_body =
            NewsletterBodyWrapper::<NewsletterTextBody>::new(dto.content.text)
                .map_err(|e| NewsletterError::ValidationError(e))?;

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
fn test_newsletter_deserialization() {
    let json_data = serde_json::json!({
        "title": "My Newsletter",
        "content": {
            "html": "<p>Hello, world!</p>",
            "text":  "Hello, world!"
        }
    });

    let newsletter_dto: Result<NewsletterDto, serde_json::Error> =
        serde_json::from_value(json_data);

    match &newsletter_dto {
        Ok(dto) => {
            assert_eq!(dto.title, "My Newsletter");
            assert_eq!(dto.content.html, "<p>Hello, world!</p>");
            assert_eq!(dto.content.text, "Hello, world!");
        }
        Err(e) => {
            panic!("Failed to deserialize: {:?}", e);
        }
    }

    let newsletter: Newsletter = NewsletterDto::try_into(newsletter_dto.unwrap()).unwrap();
    assert_eq!(newsletter.title.as_ref(), "My Newsletter");
    assert_eq!(newsletter.content.html.as_ref(), "<p>Hello, world!</p>");
    assert_eq!(newsletter.content.text.as_ref(), "Hello, world!");
}

#[test]
fn empty_newsletter_title_is_rejected() {
    let title = "".to_string();

    assert_err!(NewsletterTitle::parse(title));
}

#[test]
fn long_newsletter_title_is_rejected() {
    let title = "a".repeat(NewsletterTitle::MAX_LENGTH + 1);

    assert_err!(NewsletterTitle::parse(title));
}

#[test]
fn correct_newsletter_title_is_accepted() {
    let title = "a".repeat(NewsletterTitle::MAX_LENGTH);

    assert_eq!(
        NewsletterTitle::parse(title.clone()).unwrap().as_ref(),
        title
    );
}

#[test]
fn empty_newsletter_text_is_rejected() {
    let text_body = "".to_string();

    assert_err!(NewsletterBodyWrapper::<NewsletterTextBody>::new(text_body));
}

#[test]
fn empty_newsletter_html_is_rejected() {
    let html_body = "".to_string();

    assert_err!(NewsletterBodyWrapper::<NewsletterHtmlBody>::new(html_body));
}
#[test]
fn correct_newsletter_text_is_accepted() {
    let text_body = "a".repeat(1000);

    assert_eq!(
        NewsletterBodyWrapper::<NewsletterTextBody>::new(text_body.clone())
            .unwrap()
            .as_ref(),
        text_body
    );
}
#[test]
fn correct_newsletter_html_is_accepted() {
    let html_body = "a".repeat(1000);

    assert_eq!(
        NewsletterBodyWrapper::<NewsletterTextBody>::new(html_body.clone())
            .unwrap()
            .as_ref(),
        html_body
    );
}
