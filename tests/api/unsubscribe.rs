use crate::helpers::spawn_app; //, ConfirmationLinks, TestApp};
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};
use zero2prod::domain::new_subscriber::models::subscriber::SubscriberStatus;
use zero2prod::domain::new_subscriber::models::token::SubscriptionToken;

#[tokio::test]
async fn unsubscribe_request_without_token_is_rejected() {
    let app = spawn_app().await;

    let response = reqwest::get(&format!("{}/subscriptions/unsubscribe", app.address))
        .await
        .unwrap();

    assert_eq!(response.status().as_u16(), 400);
}

#[tokio::test]
async fn unsubscribe_request_with_incorrectly_formatted_token_is_rejected() {
    let app = spawn_app().await;

    let response = reqwest::get(&format!(
        "{}/subscriptions/unsubscribe?token=12345",
        app.address
    ))
    .await
    .unwrap();

    assert_eq!(response.status().as_u16(), 400);
}

#[tokio::test]
async fn unsubscribe_request_with_unknown_token_is_rejected_with_401() {
    let app = spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    app.post_subscriptions(body.into()).await;
    if app.confirm_subscription().await.is_none() {
        panic!("Subscription wasnt confirmed")
    }
    let token = SubscriptionToken::default();
    let response = app
        .get_subscription_unsubscribe(token.as_str().into())
        .await;
    assert_eq!(response.status().as_u16(), 401);
}

#[tokio::test]
async fn unsubscribe_request_with_unknown_subscriber_id_is_rejected_with_500() {
    let app = spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    app.post_subscriptions(body.into()).await;
    if let Some((subscriber, token)) = app.confirm_subscription().await {
        let pool = app.subscription_service.repo.pool();
        let id = subscriber.id;
        sqlx::query!("ALTER TABLE subscription_tokens DROP CONSTRAINT subscription_tokens_subscriber_id_fkey;").execute(pool).await.unwrap();
        sqlx::query!("DELETE FROM subscriptions WHERE id=$1", id)
            .execute(pool)
            .await
            .unwrap();
        let response = app
            .get_subscription_unsubscribe(token.as_str().into())
            .await;
        assert_eq!(response.status().as_u16(), 500);
    } else {
        panic!("Subscription wasnt confirmed")
    }
}
/*
#[tokio::test]
async fn sucessful_unsubscribe_request_receives_newsletter() {
    todo!()
}
    */

#[tokio::test]
async fn sucessful_unconfirmed_unsubscription_maintains_subscriber() {
    let app = spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    app.post_subscriptions(body.into()).await;
    if let Some((subscriber, token)) = app.confirm_subscription().await {
        let response = app
            .get_subscription_unsubscribe(token.as_str().into())
            .await;
        let pool = app.subscription_service.repo.pool();
        let record = sqlx::query!(
            "SELECT status FROM subscriptions WHERE email = $1",
            subscriber.email.as_str()
        )
        .fetch_one(pool)
        .await
        .unwrap();

        assert_eq!(response.status().as_u16(), 200);
        assert_eq!(
            SubscriberStatus::parse(record.status.as_str()).unwrap(),
            SubscriberStatus::CancellationPendingConfirmation
        )
    } else {
        panic!("Subscription wasnt confirmed")
    }
}
#[tokio::test]
async fn sucessful_confirmed_unsubscription_deletes_subscriber() {
    let app = spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    app.post_subscriptions(body.into()).await;
    if let Some((subscriber, token)) = app.confirm_subscription().await {
        app.get_subscription_unsubscribe(token.as_str().into())
            .await;
        let response = app
            .get_subscription_unsubscribe(token.as_str().into())
            .await;
        let pool = app.subscription_service.repo.pool();
        let record = sqlx::query!(
            "SELECT status FROM subscriptions WHERE email = $1",
            subscriber.email.as_str()
        )
        .fetch_optional(pool)
        .await
        .unwrap();

        assert_eq!(response.status().as_u16(), 200);
        assert!(record.is_none(), "Subscriber shouldn't persist in database");
    } else {
        panic!("Subscription wasnt confirmed")
    }
}
