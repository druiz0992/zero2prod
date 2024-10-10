use crate::helpers::spawn_app; //, ConfirmationLinks, TestApp};

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
/*
#[tokio::test]
async fn unsubscribe_request_with_unknown_token_is_rejected() {
    todo!()
}
#[tokio::test]
async fn unsubscribe_request_with_unknown_subscriber_id_is_rejected() {
    todo!()
}

#[tokio::test]
async fn sucessful_unsubscribe_request_receives_newsletter() {
    todo!()
}

#[tokio::test]
async fn sucessful_unsubscribe_request_redirected_to_final_confirmation() {
    todo!()
}

#[tokio::test]
async fn link_returned_in_unsubscribe_confirmation_returns_200_if_called() {
    todo!()
}
*/
