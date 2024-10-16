use crate::configuration::ApplicationSettings;
use crate::domain::new_subscriber::ports::{SubscriptionService, SubscriptionServiceError};
use crate::inbound::http::handlers::{confirm, health_check, subscribe, unsubscribe};
use crate::routes::error_chain_fmt;
use actix_web::dev::Server;
use actix_web::{http::StatusCode, ResponseError};
use actix_web::{web, App, HttpServer};
use std::net::TcpListener;
use std::sync::Arc;
use tracing_actix_web::TracingLogger;

mod handlers;
pub struct Application<SS: SubscriptionService> {
    port: u16,
    server: Server,
    subscription_service: Arc<SS>,
}

#[derive(Debug, Clone)]
struct ApplicationState<SS: SubscriptionService> {
    subscription_service: Arc<SS>,
}

fn run<SS: SubscriptionService>(
    listener: TcpListener,
    state: ApplicationState<SS>,
) -> Result<Server, std::io::Error> {
    let state = web::Data::new(state);
    let server = HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            .route("/", web::get().to(health_check))
            .route("/health_check", web::get().to(health_check))
            .route("/subscriptions", web::post().to(subscribe::<SS>))
            .route("/subscriptions/confirm", web::get().to(confirm::<SS>))
            .route(
                "/subscriptions/unsubscribe",
                web::get().to(unsubscribe::<SS>),
            )
            //.route("/newsletters", web::post().to(publish_newsletter))
            .app_data(state.clone())
    })
    .listen(listener)?
    .run();

    Ok(server)
}

impl<SS: SubscriptionService> Application<SS> {
    pub async fn build(
        subscription_service: SS,
        configuration: ApplicationSettings,
    ) -> Result<Self, std::io::Error> {
        let address = format!("{}:{}", configuration.host, configuration.port);
        let listener = TcpListener::bind(address)?;
        let port = listener.local_addr().unwrap().port();

        let subscription_service = Arc::new(subscription_service);
        let state = ApplicationState {
            subscription_service: Arc::clone(&subscription_service),
            //base_url: configuration.base_url,
        };

        let server = run(listener, state)?;
        Ok(Self {
            port,
            server,
            subscription_service,
        })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn subscription_service(&self) -> Arc<SS> {
        self.subscription_service.clone()
    }

    pub async fn run_until_stopped(self) -> Result<(), std::io::Error> {
        self.server.await
    }
}

#[derive(thiserror::Error)]
pub enum AppError {
    #[error("Subscriber Repository Error: {0}")]
    SubscriptionServiceError(SubscriptionServiceError),

    #[error("Subscriber not found")]
    SubscriberNotFound,

    #[error("There is no subscriber associated with provided token")]
    UnknownToken,

    #[error(transparent)]
    Unexpected(#[from] anyhow::Error),
}

impl From<SubscriptionServiceError> for AppError {
    fn from(error: SubscriptionServiceError) -> Self {
        match error {
            SubscriptionServiceError::Unexpected(e) => AppError::Unexpected(e),
            SubscriptionServiceError::UnknownToken => AppError::UnknownToken,
            SubscriptionServiceError::RepositorySubscriberNotFound => AppError::SubscriberNotFound,
            _ => AppError::SubscriptionServiceError(error),
        }
    }
}

impl std::fmt::Debug for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for AppError {
    fn status_code(&self) -> StatusCode {
        match self {
            AppError::SubscriptionServiceError(_) => StatusCode::BAD_REQUEST,
            AppError::Unexpected(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::SubscriberNotFound => StatusCode::NOT_FOUND,
            AppError::UnknownToken => StatusCode::UNAUTHORIZED,
        }
    }
}
