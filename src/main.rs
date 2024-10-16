use zero2prod::configuration::get_configuration;
use zero2prod::domain::new_subscriber::service::Subscription;
use zero2prod::inbound::http::Application;
use zero2prod::outbound::db::postgres_db::PostgresDb;
use zero2prod::outbound::subscription_notifier::email_client::EmailClient;
use zero2prod::outbound::telemetry::init_logger;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let configuration = get_configuration().expect("Failed to read configuration");
    init_logger("zero2prod", &configuration.log_level(), std::io::stdout);

    let email_client = EmailClient::new(configuration.email_client);
    let subscription_repo = PostgresDb::new(&configuration.database);
    let subscription_service = Subscription::new(subscription_repo, email_client);
    let application = Application::build(subscription_service, configuration.application).await?;

    application.run_until_stopped().await?;
    Ok(())
}
