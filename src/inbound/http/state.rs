use crate::domain::auth::ports::AuthService;
use crate::domain::new_subscriber::ports::SubscriptionService;
use crate::domain::newsletter::ports::NewsletterService;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct SubscriptionState<SS: SubscriptionService> {
    subscription_service: SS,
}

#[derive(Debug, Clone)]
pub struct SharedSubscriptionState<SS: SubscriptionService>(Arc<SubscriptionState<SS>>);

impl<SS: SubscriptionService> SharedSubscriptionState<SS> {
    pub fn new(subscription_service: SS) -> Self {
        Self(Arc::new(SubscriptionState {
            subscription_service,
        }))
    }
    pub fn subscription_service(&self) -> &SS {
        &self.0.subscription_service
    }
}

#[derive(Debug, Clone)]
pub struct NewsletterState<NS: NewsletterService> {
    newsletter_service: NS,
    base_url: String,
}

#[derive(Debug, Clone)]
pub struct SharedNewsletterState<NS: NewsletterService>(Arc<NewsletterState<NS>>);

impl<NS: NewsletterService> SharedNewsletterState<NS> {
    pub fn new(newsletter_service: NS, base_url: String) -> Self {
        Self(Arc::new(NewsletterState {
            newsletter_service,
            base_url,
        }))
    }
    pub fn newsletter_service(&self) -> &NS {
        &self.0.newsletter_service
    }

    pub fn url(&self) -> &str {
        &self.0.base_url
    }
}

#[derive(Debug, Clone)]
pub struct AuthState<AS: AuthService> {
    auth_service: AS,
}

#[derive(Debug, Clone)]
pub struct SharedAuthState<AS: AuthService>(Arc<AuthState<AS>>);

impl<AS: AuthService> SharedAuthState<AS> {
    pub fn new(auth_service: AS) -> Self {
        Self(Arc::new(AuthState { auth_service }))
    }

    pub fn auth_service(&self) -> &AS {
        &self.0.auth_service
    }
}
