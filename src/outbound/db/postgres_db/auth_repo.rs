use async_trait::async_trait;

use super::*;
use crate::domain::auth::credentials::{CredentialsError, StoredCredentials};
use crate::domain::auth::ports::AuthRepository;

impl PostgresDb {}

#[async_trait]
impl AuthRepository for PostgresDb {
    #[tracing::instrument(name = "Get stored credentials", skip(username, self))]
    async fn get_stored_credentials(
        &self,
        username: &str,
    ) -> Result<Option<StoredCredentials>, CredentialsError> {
        let row = sqlx::query!(
            r#"SELECT user_id, password_hash FROM users WHERE username = $1"#,
            username,
        )
        .fetch_optional(&self.pool)
        .await
        .context("Failed to perform a query to retrieve stored credentials.")?
        .map(|row| StoredCredentials::new(row.user_id, row.password_hash));
        Ok(row)
    }

    #[tracing::instrument(name = "Get username", skip(self))]
    async fn get_username(&self, user_id: uuid::Uuid) -> Result<String, anyhow::Error> {
        let row = sqlx::query!(
            r#" SELECT username FROM users WHERE user_id = $1 "#,
            user_id,
        )
        .fetch_one(&self.pool)
        .await
        .context("Failed to perform a query to retrieve a username.")?;

        Ok(row.username)
    }

    #[tracing::instrument(name = "Change password", skip(credentials, self))]
    async fn change_password(&self, credentials: StoredCredentials) -> Result<(), anyhow::Error> {
        sqlx::query!(
            r#" UPDATE users SET password_hash = $1 WHERE user_id = $2"#,
            credentials.password_hash(),
            credentials.user_id(),
        )
        .execute(&self.pool)
        .await
        .context("Failed to change user's password in the database.")?;
        Ok(())
    }
}
