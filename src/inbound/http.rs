use crate::configuration::ApplicationSettings;
use crate::domain::new_subscriber::ports::SubscriptionService;
use crate::domain::newsletter::ports::NewsletterService;
use crate::inbound::http::auth::secure_query::HmacSecret;
use crate::inbound::http::handlers::{
    confirm, health_check, home, login, login_form, publish_newsletter, subscribe, unsubscribe,
};
use actix_web::dev::Server;
use actix_web::{web, App, HttpServer};
use std::net::TcpListener;
use std::sync::Arc;
use tracing_actix_web::TracingLogger;

use actix_web::web::Data;

use secrecy::Secret;

mod auth;
mod config;
mod errors;
mod handlers;

pub struct Application<SS, NS>
where
    SS: SubscriptionService,
    NS: NewsletterService,
{
    port: u16,
    server: Server,
    subscription_state: SharedSubscriptionState<SS>,
    newsletter_state: SharedNewsletterState<NS>,
}

#[derive(Debug, Clone)]
pub struct NewsletterState<NS: NewsletterService> {
    newsletter_service: NS,
    base_url: String,
}

#[derive(Debug, Clone)]
pub struct SubscriptionState<SS: SubscriptionService> {
    subscription_service: SS,
}

pub type SharedSubscriptionState<SS> = Arc<SubscriptionState<SS>>;

impl<SS: SubscriptionService> SubscriptionState<SS> {
    fn new(subscription_service: SS) -> SharedSubscriptionState<SS> {
        Arc::new(Self {
            subscription_service,
        })
    }
    pub fn subscription_service(&self) -> &SS {
        &self.subscription_service
    }
}

pub type SharedNewsletterState<NS> = Arc<NewsletterState<NS>>;

impl<NS: NewsletterService> NewsletterState<NS> {
    fn new(newsletter_service: NS, base_url: String) -> SharedNewsletterState<NS> {
        Arc::new(Self {
            newsletter_service,
            base_url,
        })
    }
    pub fn newsletter_service(&self) -> &NS {
        &self.newsletter_service
    }
}

fn run<SS: SubscriptionService, NS: NewsletterService>(
    listener: TcpListener,
    hmac_secret: Secret<String>,
    subscription_state: SharedSubscriptionState<SS>,
    newsletter_state: SharedNewsletterState<NS>,
) -> Result<Server, std::io::Error> {
    let subscription_state = web::Data::new(subscription_state);
    let newsletter_state = web::Data::new(newsletter_state);

    let server = HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            .route("/", web::get().to(home))
            .route("/health_check", web::get().to(health_check))
            .app_data(newsletter_state.clone())
            .route("/newsletters", web::post().to(publish_newsletter::<NS>))
            .route("/login", web::get().to(login_form))
            .route("/login", web::post().to(login::<NS>))
            .app_data(subscription_state.clone())
            .route("/subscriptions", web::post().to(subscribe::<SS>))
            .route("/subscriptions/confirm", web::get().to(confirm::<SS>))
            .route(
                "/subscriptions/unsubscribe",
                web::get().to(unsubscribe::<SS>),
            )
            .app_data(Data::new(HmacSecret(hmac_secret.clone())))
    })
    .listen(listener)?
    .run();

    Ok(server)
}

impl<SS: SubscriptionService, NS: NewsletterService> Application<SS, NS> {
    pub async fn build(
        subscription_service: SS,
        newsletter_service: NS,
        configuration: ApplicationSettings,
    ) -> Result<Self, std::io::Error> {
        let address = format!("{}:{}", configuration.host, configuration.port);
        let listener = TcpListener::bind(address)?;
        let port = listener.local_addr().unwrap().port();

        let newsletter_state = NewsletterState::new(newsletter_service, configuration.base_url);
        let subscription_state = SubscriptionState::new(subscription_service);

        let server: Server = run(
            listener,
            configuration.hmac_secret,
            Arc::clone(&subscription_state),
            Arc::clone(&newsletter_state),
        )?;

        Ok(Self {
            port,
            server,
            subscription_state: Arc::clone(&subscription_state),
            newsletter_state: Arc::clone(&newsletter_state),
        })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn subscription_state(&self) -> SharedSubscriptionState<SS> {
        self.subscription_state.clone()
    }

    pub fn newsletter_state(&self) -> SharedNewsletterState<NS> {
        self.newsletter_state.clone()
    }

    pub async fn run_until_stopped(self) -> Result<(), std::io::Error> {
        self.server.await
    }
}
