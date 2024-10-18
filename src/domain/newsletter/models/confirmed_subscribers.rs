use crate::domain::new_subscriber::models::{
    email::SubscriberEmail,
    subscriber::{NewSubscriber, SubscriberStatus},
};
pub struct ConfirmedSubscriber(NewSubscriber);

impl ConfirmedSubscriber {
    pub fn new(subscriber: NewSubscriber) -> Result<Self, String> {
        if subscriber.status == SubscriberStatus::SubscriptionConfirmed {
            Ok(ConfirmedSubscriber(subscriber))
        } else {
            Err("Subscriber must be confirmed".to_string())
        }
    }

    pub fn email(&self) -> &SubscriberEmail {
        &self.0.email
    }
}
