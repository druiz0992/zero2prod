//mod newsletters;

use crate::domain::new_subscriber::models::token::SubscriptionToken;
use sqlx::PgPool;

pub fn error_chain_fmt(
    e: &impl std::error::Error,
    f: &mut std::fmt::Formatter<'_>,
) -> std::fmt::Result {
    writeln!(f, "{}\n", e)?;
    let mut current = e.source();
    while let Some(cause) = current {
        writeln!(f, "Caused by:\n\t{}", cause)?;
        current = cause.source();
    }
    Ok(())
}

#[tracing::instrument(name = "Get token from subscriber id", skip(pool, subscriber_id))]
async fn get_token_from_subscriber_id(
    pool: &PgPool,
    subscriber_id: uuid::Uuid,
) -> Result<SubscriptionToken, anyhow::Error> {
    let result = sqlx::query!(
        r#"SELECT subscription_token FROM subscription_tokens WHERE  subscriber_id= $1"#,
        subscriber_id
    )
    .fetch_one(pool)
    .await?;

    SubscriptionToken::try_from(result.subscription_token)
        .map_err(|e| anyhow::Error::msg(format!("{}", e)))
}
