use crate::configuration::ApplicationSettings;
use crate::domain::auth::ports::AuthService;
use crate::domain::new_subscriber::ports::SubscriptionService;
use crate::domain::newsletter::ports::NewsletterService;
use crate::inbound::http::handlers::{
    admin::change_password, admin::change_password_form, admin_dashboard, confirm, health_check,
    home, log_out, login, login_form, publish_newsletter, subscribe, unsubscribe,
};
use crate::inbound::http::state::{
    SharedAuthState, SharedNewsletterState, SharedSubscriptionState,
};
use actix_session::storage::RedisSessionStore;
use actix_session::SessionMiddleware;
use actix_web::cookie::Key;
use actix_web::dev::Server;
use actix_web::{web, App, HttpServer};
use actix_web_flash_messages::storage::CookieMessageStore;
use actix_web_flash_messages::FlashMessagesFramework;
use actix_web_lab::middleware::from_fn;
use auth::reject_anonymous_users;
use std::net::TcpListener;
use tracing_actix_web::TracingLogger;

use secrecy::{ExposeSecret, Secret};

mod auth;
mod errors;
mod handlers;
pub mod state;
mod utils;

pub struct Application<SS, NS, AS>
where
    SS: SubscriptionService,
    NS: NewsletterService,
    AS: AuthService,
{
    port: u16,
    server: Server,
    subscription_state: SharedSubscriptionState<SS>,
    newsletter_state: SharedNewsletterState<NS>,
    auth_state: SharedAuthState<AS>,
}

async fn run<SS: SubscriptionService, NS: NewsletterService, AS: AuthService>(
    listener: TcpListener,
    hmac_secret: Secret<String>,
    redis_uri: Secret<String>,
    subscription_state: SharedSubscriptionState<SS>,
    newsletter_state: SharedNewsletterState<NS>,
    auth_state: SharedAuthState<AS>,
) -> Result<Server, anyhow::Error> {
    let subscription_state = web::Data::new(subscription_state);
    let newsletter_state = web::Data::new(newsletter_state);
    let auth_state = web::Data::new(auth_state);

    let secret_key = Key::from(hmac_secret.expose_secret().as_bytes());
    let message_store = CookieMessageStore::builder(secret_key.clone()).build();
    let message_framework = FlashMessagesFramework::builder(message_store).build();
    let redis_store = RedisSessionStore::new(redis_uri.expose_secret()).await?;

    let server = HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            .wrap(message_framework.clone())
            .wrap(SessionMiddleware::new(
                redis_store.clone(),
                secret_key.clone(),
            ))
            .route("/", web::get().to(home))
            .route("/health_check", web::get().to(health_check))
            .app_data(auth_state.clone())
            .app_data(newsletter_state.clone())
            .route("/newsletters", web::post().to(publish_newsletter::<NS, AS>))
            .route("/login", web::get().to(login_form))
            .route("/login", web::post().to(login::<AS>))
            .app_data(subscription_state.clone())
            .route("/subscriptions", web::post().to(subscribe::<SS>))
            .route("/subscriptions/confirm", web::get().to(confirm::<SS>))
            .route(
                "/subscriptions/unsubscribe",
                web::get().to(unsubscribe::<SS>),
            )
            .service(
                web::scope("/admin")
                    .wrap(from_fn(reject_anonymous_users))
                    .app_data(auth_state.clone())
                    .route("/dashboard", web::get().to(admin_dashboard::<AS>))
                    .route("/password", web::get().to(change_password_form))
                    .route("/password", web::post().to(change_password::<AS>))
                    .route("/logout", web::post().to(log_out)),
            )
    })
    .listen(listener)?
    .run();

    Ok(server)
}

impl<SS, NS, AS> Application<SS, NS, AS>
where
    SS: SubscriptionService,
    NS: NewsletterService,
    AS: AuthService,
{
    pub async fn build(
        subscription_service: SS,
        newsletter_service: NS,
        auth_service: AS,
        configuration: ApplicationSettings,
    ) -> Result<Self, anyhow::Error> {
        let address = format!("{}:{}", configuration.host, configuration.port);
        let listener = TcpListener::bind(address)?;
        let port = listener.local_addr().unwrap().port();

        let newsletter_state =
            SharedNewsletterState::new(newsletter_service, configuration.base_url);
        let subscription_state = SharedSubscriptionState::new(subscription_service);
        let auth_state = SharedAuthState::new(auth_service);

        let server: Server = run(
            listener,
            configuration.hmac_secret,
            configuration.redis_uri,
            subscription_state.clone(),
            newsletter_state.clone(),
            auth_state.clone(),
        )
        .await?;

        Ok(Self {
            port,
            server,
            subscription_state: subscription_state.clone(),
            newsletter_state: newsletter_state.clone(),
            auth_state: auth_state.clone(),
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

    pub fn auth_state(&self) -> SharedAuthState<AS> {
        self.auth_state.clone()
    }

    pub async fn run_until_stopped(self) -> Result<(), std::io::Error> {
        self.server.await
    }
}
