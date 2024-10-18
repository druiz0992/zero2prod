use zero2prod::configuration::get_configuration;
use zero2prod::domain::new_subscriber::service::Subscription;
use zero2prod::domain::newsletter::service::Blog;
use zero2prod::inbound::http::Application;
use zero2prod::outbound::db::postgres_db::PostgresDb;
use zero2prod::outbound::notifier::email_client::EmailClient;
use zero2prod::outbound::telemetry::init_logger;

use std::sync::Arc;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let configuration = get_configuration().expect("Failed to read configuration");
    init_logger("zero2prod", &configuration.log_level(), std::io::stdout);

    let email_client = Arc::new(EmailClient::new(configuration.email_client));
    let repo = Arc::new(PostgresDb::new(&configuration.database));
    let newsletter_service = Blog::new(Arc::clone(&repo), Arc::clone(&email_client));
    let subscription_service = Subscription::new(Arc::clone(&repo), Arc::clone(&email_client));
    let application = Application::build(
        subscription_service,
        newsletter_service,
        configuration.application,
    )
    .await?;

    application.run_until_stopped().await?;
    Ok(())
}
