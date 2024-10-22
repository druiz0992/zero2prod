use crate::helpers::{assert_is_redirect_to, spawn_app, ConfirmationLinks, TestApp};
use wiremock::matchers::{any, method, path};
use wiremock::{Mock, ResponseTemplate};

async fn create_unconfirmed_subscriber(app: &TestApp) -> ConfirmationLinks {
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    let _mock_guard = Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .named("Create unconfirmed subscriber")
        .expect(1)
        .mount_as_scoped(&app.email_server)
        .await;

    app.post_subscriptions(body.into())
        .await
        .error_for_status()
        .unwrap();

    let email_request = &app.get_email_requests().await;
    app.get_confirmation_links(&email_request)
}

async fn create_confirmed_subscriber(app: &TestApp) {
    let confirmation_links = create_unconfirmed_subscriber(app).await;
    reqwest::get(confirmation_links.html)
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
}

#[derive(Clone, Debug)]
struct Newsletter(serde_json::Value);
impl Newsletter {
    fn new() -> Newsletter {
        Newsletter(serde_json::json!({}))
    }
    fn title(mut self) -> Newsletter {
        let title_value = "Newsletter title";
        self.0.as_object_mut().unwrap().insert(
            "title".to_string(),
            serde_json::Value::String(title_value.into()),
        );
        self
    }
    fn text_body(mut self) -> Newsletter {
        let text_body_value = "Newsletter body as plain text.";
        self.0.as_object_mut().unwrap().insert(
            "text_content".to_string(),
            serde_json::Value::String(text_body_value.into()),
        );
        self
    }
    fn html_body(mut self) -> Newsletter {
        let html_body_value = "<p>Newsletter body as HTML.";
        self.0.as_object_mut().unwrap().insert(
            "html_content".to_string(),
            serde_json::Value::String(html_body_value.into()),
        );
        self
    }
    fn inner(self) -> serde_json::Value {
        self.0
    }
}

fn build_newsletter() -> serde_json::Value {
    serde_json::json!({
        "title": "Newsletter title",
        "text_content": "Newsletter body as plain text",
        "html_content": "<p>Newsletter body as HTML</p>",
    })
}

#[tokio::test]
async fn newsletters_are_not_delivered_to_unconfirmed_subscribers() {
    // Arrange
    let app = spawn_app().await;
    create_unconfirmed_subscriber(&app).await;
    app.test_user.login(&app).await;

    Mock::given(any())
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&app.email_server)
        .await;

    // Act
    let newsletter_request_body = build_newsletter();
    let response = app.post_publish_newsletter(&newsletter_request_body).await;
    assert_is_redirect_to(&response, "/admin/newsletters");

    // Assert
    let html_page = app.get_publish_newsletter_html().await;
    assert!(html_page.contains("<p><i>The newsletter issue has been published!</i></p>"));
    // Mock verifies on Drop that we haven't sent the newsletter email
}

#[tokio::test]
async fn newsletters_are_delivered_to_confirmed_subscribers() {
    let app = spawn_app().await;
    create_confirmed_subscriber(&app).await;
    app.test_user.login(&app).await;

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    let newsletter_request_body = build_newsletter();

    let response = app.post_publish_newsletter(&newsletter_request_body).await;
    assert_is_redirect_to(&response, "/admin/newsletters");

    let html_page = app.get_publish_newsletter_html().await;
    assert!(html_page.contains("<p><i>The newsletter issue has been published!</i></p>"));
}

#[tokio::test]
async fn newsletter_returns_400_for_invalid_data() {
    let app = spawn_app().await;
    create_confirmed_subscriber(&app).await;
    app.test_user.login(&app).await;

    let test_cases = vec![
        (Newsletter::new(), "Empty newsletter"),
        (Newsletter::new().title(), "Missing content"),
        (
            Newsletter::new().text_body(),
            "Missing title and HTML content",
        ),
        (
            Newsletter::new().html_body(),
            "Missing title and text content",
        ),
        (Newsletter::new().title().html_body(), "Missing text"),
        (Newsletter::new().text_body(), "Missing HTML and title"),
        (Newsletter::new().text_body().html_body(), "Missing title"),
        (
            Newsletter::new().title().title().html_body(),
            "Missing Text",
        ),
        (
            Newsletter::new().title().title().text_body(),
            "Missing HTML",
        ),
    ];

    for (invalid_body, error_message) in test_cases {
        dbg!(&invalid_body.clone().inner());
        let response = app.post_publish_newsletter(&invalid_body.inner()).await;

        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not fail with 400 Bad Request when the payload was {}.",
            error_message
        );
    }
}

#[tokio::test]
async fn both_unsubscribe_links_in_newsletter_return_a_200_if_called() {
    let app: TestApp = spawn_app().await;
    create_confirmed_subscriber(&app).await;
    let response = app.test_user.login(&app).await;
    assert_is_redirect_to(&response, "/admin/dashboard");

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    let newsletter_request_body = build_newsletter();

    let response = app.post_publish_newsletter(&newsletter_request_body).await;
    assert_is_redirect_to(&response, "/admin/newsletters");

    let email_newsletter = &app.get_email_requests().await;
    let confirmation_links = app.get_newsletter_unsubscribe_links(&email_newsletter);

    let response_text = reqwest::Client::new()
        .get(&format!("{}", confirmation_links.plain_text))
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(
        200,
        response_text.status().as_u16(),
        "Checking unsubscribe link in text newsletter {}",
        confirmation_links.plain_text
    );

    let response_html = reqwest::Client::new()
        .get(&format!("{}", confirmation_links.html))
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(
        200,
        response_html.status().as_u16(),
        "Checking unsubscribe link in HTML newsletter {}",
        confirmation_links.html
    );
}
