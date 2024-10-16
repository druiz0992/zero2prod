use crate::helpers::spawn_app;
use futures::stream::{self, StreamExt};
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};
use zero2prod::domain::new_subscriber::models::subscriber::SubscriberStatus;

#[tokio::test]
async fn confirmation_without_token_is_rejected_with_a_400() {
    let app = spawn_app().await;

    let response = reqwest::get(&format!("{}/subscriptions/confirm", app.address))
        .await
        .unwrap();

    assert_eq!(response.status().as_u16(), 400);
}

#[tokio::test]
async fn confirmation_with_invalid_token_is_rejected_with_a_400() {
    let app = spawn_app().await;

    let response = reqwest::get(&format!(
        "{}/subscriptions/confirm?subscription_token=1233",
        app.address
    ))
    .await
    .unwrap();

    assert_eq!(response.status().as_u16(), 400);
}

#[tokio::test]
async fn the_link_returned_by_subscribe_returns_a_200_if_called() {
    let app = spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    app.post_subscriptions(body.into()).await;
    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let confirmation_links = app.get_confirmation_links(&email_request);

    let response = reqwest::get(confirmation_links.html).await.unwrap();

    assert_eq!(response.status().as_u16(), 200);
}

#[tokio::test]
async fn both_links_returned_by_subscribe_return_a_200_if_called() {
    let app = spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(2)
        .mount(&app.email_server)
        .await;

    app.post_subscriptions(body.into()).await;
    app.post_subscriptions(body.into()).await;

    let email_requests = &app.email_server.received_requests().await.unwrap();
    stream::iter(email_requests.into_iter())
        .for_each_concurrent(None, |r| async {
            let confirmation_links = app.get_confirmation_links(r);

            let response = reqwest::get(confirmation_links.html).await.unwrap();

            assert_eq!(response.status().as_u16(), 200);
        })
        .await;
}

#[tokio::test]
async fn clicking_on_the_confirmation_link_confirms_a_subscriber() {
    let app = spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    app.post_subscriptions(body.into()).await;
    if let Some((subscriber, _)) = app.confirm_subscription().await {
        assert_eq!(subscriber.name.as_ref(), "le guin");
        assert_eq!(subscriber.email.as_ref(), "ursula_le_guin@gmail.com");
        assert_eq!(subscriber.status, SubscriberStatus::SubscriptionConfirmed);
    } else {
        panic!("Subscription wasnt confirmed")
    }
}

#[tokio::test]
async fn clicking_on_the_confirmation_link_twice_confirms_a_subscriber() {
    let app = spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    app.post_subscriptions(body.into()).await;
    if app.confirm_subscription().await.is_none() {
        panic!("Subscription wasn't confirmed the first time ")
    }
    if let Some((subscriber, _)) = app.confirm_subscription().await {
        assert_eq!(subscriber.name.as_ref(), "le guin");
        assert_eq!(subscriber.email.as_ref(), "ursula_le_guin@gmail.com");
        assert_eq!(subscriber.status, SubscriberStatus::SubscriptionConfirmed);
    } else {
        panic!("Subscription wasnt confirmed the second time")
    }
}

#[tokio::test]
async fn if_the_link_returned_by_subscribe_doesnt_exist_return_401() {
    let app = spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    app.post_subscriptions(body.into()).await;
    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let confirmation_links = app.get_confirmation_links(&email_request);
    let token = confirmation_links
        .html
        .query()
        .unwrap()
        .split("=")
        .nth(1)
        .unwrap();

    if let Some(_) = confirmation_links.html.query() {
        let pool = app.subscription_service.repo.pool();
        sqlx::query!(
            "DELETE FROM subscription_tokens WHERE subscription_token=$1",
            token
        )
        .execute(pool)
        .await
        .unwrap();
    }

    let response = reqwest::get(confirmation_links.html).await.unwrap();

    assert_eq!(response.status().as_u16(), 401);
}
