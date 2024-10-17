use crate::configuration::ApplicationSettings;
use crate::domain::new_subscriber::errors::SubscriberError;
use crate::domain::new_subscriber::ports::SubscriptionService;
use crate::domain::newsletter::errors::NewsletterError;
use crate::domain::newsletter::ports::NewsletterService;
use crate::inbound::http::handlers::{
    confirm, health_check, publish_newsletter, subscribe, unsubscribe,
};
use crate::routes::error_chain_fmt;
use actix_web::dev::Server;
use actix_web::{http::StatusCode, ResponseError};
use actix_web::{web, App, HttpServer};
use std::net::TcpListener;
use std::sync::Arc;
use tracing_actix_web::TracingLogger;

use actix_web::http::header;
use actix_web::http::header::HeaderValue;
use actix_web::HttpResponse;

mod auth;
mod handlers;
pub struct Application<SS, NS>
where
    SS: SubscriptionService,
    NS: NewsletterService,
{
    port: u16,
    server: Server,
    subscription_service: Arc<SS>,
    newsletter_service: Arc<NS>,
}

#[derive(Debug, Clone)]
struct NewsletterState<NS: NewsletterService> {
    newsletter_service: Arc<NS>,
    base_url: String,
}

#[derive(Debug, Clone)]
struct SubscriptionState<SS: SubscriptionService> {
    subscription_service: Arc<SS>,
}

fn run<SS: SubscriptionService, NS: NewsletterService>(
    listener: TcpListener,
    subscription_state: SubscriptionState<SS>,
    newsletter_state: NewsletterState<NS>,
) -> Result<Server, std::io::Error> {
    let subscription_state = web::Data::new(subscription_state);
    let newsletter_state = web::Data::new(newsletter_state);

    let server = HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            .route("/", web::get().to(health_check))
            .route("/health_check", web::get().to(health_check))
            .app_data(newsletter_state.clone())
            .route("/newsletters", web::post().to(publish_newsletter::<NS>))
            .app_data(subscription_state.clone())
            .route("/subscriptions", web::post().to(subscribe::<SS>))
            .route("/subscriptions/confirm", web::get().to(confirm::<SS>))
            .route(
                "/subscriptions/unsubscribe",
                web::get().to(unsubscribe::<SS>),
            )
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

        let newsletter_service = Arc::new(newsletter_service);
        let newsletter_state = NewsletterState {
            newsletter_service: Arc::clone(&newsletter_service),
            base_url: configuration.base_url,
        };

        let subscription_service = Arc::new(subscription_service);
        let subscription_state = SubscriptionState {
            subscription_service: Arc::clone(&subscription_service),
        };

        let server = run(listener, subscription_state, newsletter_state)?;
        Ok(Self {
            port,
            server,
            subscription_service,
            newsletter_service,
        })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn subscription_service(&self) -> Arc<SS> {
        self.subscription_service.clone()
    }

    pub fn newsletter_service(&self) -> Arc<NS> {
        self.newsletter_service.clone()
    }

    pub async fn run_until_stopped(self) -> Result<(), std::io::Error> {
        self.server.await
    }
}

#[derive(thiserror::Error, Debug)]
pub enum AppError {
    #[error("Validation error: {0}")]
    ValidationError(String),
    #[error("Subscriber not found: {0}")]
    NotFound(String),
    #[error("Subscriber not authenticated: {0}")]
    AuthError(String),
    #[error(transparent)]
    Unexpected(#[from] anyhow::Error),
}

impl From<SubscriberError> for AppError {
    fn from(error: SubscriberError) -> Self {
        match error {
            SubscriberError::ValidationError(s) => AppError::ValidationError(s),
            SubscriberError::AuthError(s) => AppError::AuthError(s),
            SubscriberError::NotFound(s) => AppError::NotFound(s),
            SubscriberError::Unexpected(s) => AppError::Unexpected(s),
        }
    }
}

impl From<NewsletterError> for AppError {
    fn from(error: NewsletterError) -> Self {
        match error {
            NewsletterError::ValidationError(s) => AppError::ValidationError(s),
            NewsletterError::NotFound(s) => AppError::NotFound(s),
            NewsletterError::Unexpected(s) => AppError::Unexpected(s),
            NewsletterError::AuthError(s) => AppError::AuthError(s),
        }
    }
}

impl ResponseError for AppError {
    fn status_code(&self) -> StatusCode {
        match self {
            AppError::ValidationError(_) => StatusCode::BAD_REQUEST,
            AppError::Unexpected(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::AuthError(_) => StatusCode::UNAUTHORIZED,
        }
    }

    fn error_response(&self) -> actix_web::HttpResponse<actix_web::body::BoxBody> {
        match self {
            AppError::ValidationError(_) => HttpResponse::new(StatusCode::BAD_REQUEST),
            AppError::Unexpected(_) => HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR),
            AppError::NotFound(_) => HttpResponse::new(StatusCode::NOT_FOUND),
            AppError::AuthError(_) => {
                let mut response = HttpResponse::new(StatusCode::UNAUTHORIZED);
                let header_value = HeaderValue::from_str(r#"Basic realm="publish""#).unwrap();
                response
                    .headers_mut()
                    .insert(header::WWW_AUTHENTICATE, header_value);
                response
            }
        }
    }
}
