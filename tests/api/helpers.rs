use argon2::password_hash::SaltString;
use argon2::{Algorithm, Argon2, Params, PasswordHasher, Version};
use once_cell::sync::Lazy;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use std::sync::Arc;
use uuid::Uuid;
use wiremock::MockServer;
use zero2prod::domain::auth::service::BlogAuth;
use zero2prod::domain::new_subscriber::{
    models::{subscriber::NewSubscriber, token::SubscriptionToken},
    ports::SubscriberRepository,
    service::BlogSubscription,
};
use zero2prod::domain::newsletter::service::BlogDelivery;
use zero2prod::inbound::http::state::{SharedNewsletterState, SharedSubscriptionState};
use zero2prod::inbound::http::Application;
use zero2prod::outbound::{db::postgres_db::PostgresDb, notifier::email_client::EmailClient};
use zero2prod::{
    configuration::{get_configuration, DatabaseSettings},
    outbound::telemetry::init_logger,
};

pub struct TestUser {
    pub user_id: Uuid,
    pub username: String,
    pub password: String,
}

impl TestUser {
    pub fn generate() -> Self {
        Self {
            user_id: Uuid::new_v4(),
            username: Uuid::new_v4().to_string(),
            password: Uuid::new_v4().to_string(),
        }
    }

    pub async fn login(&self, app: &TestApp) -> reqwest::Response {
        app.post_login(&serde_json::json!({
            "username": &self.username,
            "password": &self.password
        }))
        .await
    }

    async fn store(&self, db: Arc<PostgresDb>) {
        let db = db.as_ref();
        let salt = SaltString::generate(&mut rand::thread_rng());
        let password_hash = Argon2::new(
            Algorithm::Argon2id,
            Version::V0x13,
            Params::new(15000, 2, 1, None).unwrap(),
        )
        .hash_password(self.password.as_bytes(), &salt)
        .unwrap()
        .to_string();
        sqlx::query!(
            "INSERT INTO users (user_id, username, password_hash) VALUES ($1, $2, $3)",
            self.user_id,
            self.username,
            password_hash
        )
        .execute(db.pool())
        .await
        .expect("Failed to store test user.");
    }
}

#[derive(Debug)]
pub struct ConfirmationLinks {
    pub html: reqwest::Url,
    pub plain_text: reqwest::Url,
}

pub struct TestApp {
    pub address: String,
    pub subscription_state: SharedSubscriptionState<BlogSubscription<PostgresDb, EmailClient>>,
    #[allow(dead_code)]
    pub newsletter_state: SharedNewsletterState<BlogDelivery<PostgresDb, EmailClient>>,
    pub email_server: MockServer,
    pub port: u16,
    pub test_user: TestUser,
    pub api_client: reqwest::Client,
}

impl TestApp {
    pub fn subscription_service(&self) -> &BlogSubscription<PostgresDb, EmailClient> {
        self.subscription_state.subscription_service()
    }

    pub fn subscription_repo(&self) -> Arc<PostgresDb> {
        let subscription_service = self.subscription_service();
        subscription_service.repo.clone()
    }

    pub async fn post_subscriptions(&self, body: String) -> reqwest::Response {
        self.api_client
            .post(&format!("{}/subscriptions", &self.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn get_subscription_unsubscribe(&self, token: String) -> reqwest::Response {
        self.api_client
            .get(&format!("{}/subscriptions/unsubscribe", &self.address))
            .query(&[("subscription_token", token.as_str())])
            .header("Content-Type", "application/x-www-form-urlencoded")
            .send()
            .await
            .expect("Failed to execute unsubscription request.")
    }

    pub fn get_confirmation_links(&self, email_requests: &wiremock::Request) -> ConfirmationLinks {
        let body: serde_json::Value = serde_json::from_slice(&email_requests.body).unwrap();
        let get_link = |s: &str| {
            let links: Vec<_> = linkify::LinkFinder::new()
                .links(s)
                .filter(|l| *l.kind() == linkify::LinkKind::Url)
                .collect();
            assert_eq!(links.len(), 1);
            let raw_link = links[0].as_str().to_owned();
            let mut confirmation_link = reqwest::Url::parse(&raw_link).unwrap();
            assert_eq!(confirmation_link.host_str().unwrap(), "127.0.0.1");
            confirmation_link.set_port(Some(self.port)).unwrap();
            confirmation_link
        };

        let html = get_link(&body["HtmlBody"].as_str().unwrap());
        let plain_text = get_link(&body["TextBody"].as_str().unwrap());
        ConfirmationLinks { html, plain_text }
    }

    pub fn get_newsletter_unsubscribe_links(
        &self,
        email_requests: &wiremock::Request,
    ) -> ConfirmationLinks {
        self.get_confirmation_links(&email_requests)
    }

    pub async fn get_email_requests(&self) -> wiremock::Request {
        self.email_server
            .received_requests()
            .await
            .unwrap()
            .pop()
            .unwrap()
    }

    pub async fn confirm_subscription(&self) -> Option<(NewSubscriber, SubscriptionToken)> {
        let email_request = &self.email_server.received_requests().await.unwrap()[0];
        let confirmation_links = self.get_confirmation_links(&email_request);
        let token = confirmation_links
            .html
            .query()
            .unwrap()
            .split("=")
            .nth(1)
            .unwrap();
        let token = SubscriptionToken::parse(token.into()).unwrap();

        reqwest::get(confirmation_links.html)
            .await
            .unwrap()
            .error_for_status()
            .unwrap();

        let saved = self
            .subscription_service()
            .repo
            .retrieve_from_token(&token)
            .await
            .expect("Failed to fetch saved subscription.");

        Some((saved, token))
    }

    pub async fn post_login<Body>(&self, body: &Body) -> reqwest::Response
    where
        Body: serde::Serialize,
    {
        self.api_client
            .post(&format!("{}/login", &self.address))
            .form(body)
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn get_login_html(&self) -> String {
        self.api_client
            .get(&format!("{}/login", &self.address))
            .send()
            .await
            .expect("Failed to execute request.")
            .text()
            .await
            .unwrap()
    }

    pub async fn get_admin_dashboard(&self) -> reqwest::Response {
        self.api_client
            .get(&format!("{}/admin/dashboard", &self.address))
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn get_admin_dashboard_html(&self) -> String {
        self.get_admin_dashboard().await.text().await.unwrap()
    }

    pub async fn get_change_password(&self) -> reqwest::Response {
        self.api_client
            .get(&format!("{}/admin/password", &self.address))
            .send()
            .await
            .expect("Failed to execute request.")
    }
    pub async fn post_change_password<Body>(&self, body: &Body) -> reqwest::Response
    where
        Body: serde::Serialize,
    {
        self.api_client
            .post(&format!("{}/admin/password", &self.address))
            .form(body)
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn get_change_password_html(&self) -> String {
        self.get_change_password().await.text().await.unwrap()
    }

    pub async fn post_logout(&self) -> reqwest::Response {
        self.api_client
            .post(&format!("{}/admin/logout", &self.address))
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn get_publish_newsletter(&self) -> reqwest::Response {
        self.api_client
            .get(&format!("{}/admin/newsletters", &self.address))
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn get_publish_newsletter_html(&self) -> String {
        self.get_publish_newsletter().await.text().await.unwrap()
    }

    pub async fn post_publish_newsletter<Body>(&self, body: &Body) -> reqwest::Response
    where
        Body: serde::Serialize,
    {
        self.api_client
            .post(&format!("{}/admin/newsletters", &self.address))
            .form(body)
            .send()
            .await
            .expect("Failed to execute request.")
    }
}

static TRACING: Lazy<()> = Lazy::new(|| {
    let c = get_configuration().expect("Failed to read configuration");
    let default_filter_level = c.general.log_level;
    let subscriber_name = "test".to_string();
    if std::env::var("TEST_LOG").is_ok() {
        init_logger(&subscriber_name, &default_filter_level, std::io::stdout);
    } else {
        init_logger(&subscriber_name, &default_filter_level, std::io::sink);
    }
});

pub async fn spawn_app() -> TestApp {
    Lazy::force(&TRACING);
    let email_server = MockServer::start().await;
    let configuration = {
        let mut c = get_configuration().expect("Failed to read configuration");
        c.database.database_name = Uuid::new_v4().to_string();
        c.application.port = 0;
        c.email_client.base_url = email_server.uri();
        c
    };

    configure_database(&configuration.database).await;

    let email_client = Arc::new(EmailClient::new(configuration.email_client));
    let repo = Arc::new(PostgresDb::new(&configuration.database));
    let subscription_service = BlogSubscription::new(Arc::clone(&repo), Arc::clone(&email_client));
    let newsletter_service = BlogDelivery::new(Arc::clone(&repo), Arc::clone(&email_client));
    let auth_service = BlogAuth::new(Arc::clone(&repo));

    let application = Application::build(
        subscription_service,
        newsletter_service,
        auth_service,
        configuration.application.clone(),
    )
    .await
    .expect("Failed to build application");

    let application_port = application.port();
    let subscription_state = application.subscription_state();
    let newsletter_state = application.newsletter_state();

    let _ = tokio::spawn(application.run_until_stopped());

    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .cookie_store(true)
        .build()
        .unwrap();

    let test_app = TestApp {
        address: format!("http://localhost:{}", application_port),
        port: application_port,
        subscription_state,
        newsletter_state,
        email_server,
        test_user: TestUser::generate(),
        api_client: client,
    };

    test_app.test_user.store(repo.clone()).await;
    test_app
}

pub async fn configure_database(config: &DatabaseSettings) -> PgPool {
    let mut connection = PgConnection::connect_with(&config.without_db())
        .await
        .expect("Failed to connect to Postgres");
    connection
        .execute(format!(r#"CREATE DATABASE "{}";"#, config.database_name).as_str())
        .await
        .expect("Failed to create database");

    let connection_pool = PgPool::connect_with(config.with_db())
        .await
        .expect("Failed to connect to Postgres");
    sqlx::migrate!("./migrations")
        .run(&connection_pool)
        .await
        .expect("Failed to migrated database");

    connection_pool
}

pub fn assert_is_redirect_to(response: &reqwest::Response, location: &str) {
    assert_eq!(response.status().as_u16(), 303);
    assert_eq!(response.headers().get("Location").unwrap(), location);
}
