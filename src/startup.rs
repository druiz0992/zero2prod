use crate::configuration::DatabaseSettings;
use crate::configuration::Settings;
use crate::domain::new_subscriber::ports::SubscriptionRepository;
use crate::domain::new_subscriber::ports::SubscriptionService;
use crate::domain::new_subscriber::service::Subscription;
use crate::email_client::EmailClient;
use crate::outbound::db::postgres_db::PostgresDb;
use crate::routes::{confirm, health_check, publish_newsletter, subscribe, unsubscribe};
use actix_web::dev::Server;
use actix_web::web::Data;
use actix_web::{web, App, HttpServer};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::net::TcpListener;
use std::sync::Arc;
use tracing_actix_web::TracingLogger;

pub struct Application {
    port: u16,
    server: Server,
}

#[derive(Clone)]
pub struct ApplicationBaseUrl(pub String);

pub fn run(
    listener: TcpListener,
    subscription_service: Box<dyn SubscriptionService>,
    email_client: EmailClient,
    base_url: String,
) -> Result<Server, std::io::Error> {
    let subscription_service = web::Data::new(subscription_service);
    let email_client = web::Data::new(email_client);
    let base_url = Data::new(base_url);
    let server = HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            .route("/", web::get().to(health_check))
            .route("/health_check", web::get().to(health_check))
            .route("/subscriptions", web::post().to(subscribe))
            .route("/subscriptions/confirm", web::get().to(confirm))
            .route("/subscriptions/unsubscribe", web::get().to(unsubscribe))
            .route("/newsletters", web::post().to(publish_newsletter))
            .app_data(subscription_service.clone())
            .app_data(base_url.clone())
            .app_data(email_client.clone())
    })
    .listen(listener)?
    .run();

    Ok(server)
}

impl Application {
    pub async fn build(configuration: Settings) -> Result<Self, std::io::Error> {
        let connection_pool = PostgresDb::new(&configuration.database);

        let sender_email = configuration
            .email_client
            .sender()
            .expect("Invalid sender email address");
        let timeout = configuration.email_client.timeout();
        let email_client = EmailClient::new(
            configuration.email_client.base_url,
            sender_email,
            configuration.email_client.authorization_token,
            timeout,
        );

        let address = format!(
            "{}:{}",
            configuration.application.host, configuration.application.port
        );
        let listener = TcpListener::bind(address)?;
        let port = listener.local_addr().unwrap().port();
        let subscription_repo = connection_pool;
        let subscription_service = Subscription::new(subscription_repo);

        let server = run(
            listener,
            Box::new(subscription_service),
            email_client,
            configuration.application.base_url,
        )?;
        Ok(Self { port, server })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub async fn run_until_stopped(self) -> Result<(), std::io::Error> {
        self.server.await
    }
}

pub fn get_connection_pool(configuration: &DatabaseSettings) -> PgPool {
    PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_secs(2))
        .connect_lazy_with(configuration.with_db())
}
