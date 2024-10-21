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
}
