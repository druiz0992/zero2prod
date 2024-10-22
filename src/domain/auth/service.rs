use super::{
    credentials::{compute_password_hash, StoredCredentials},
    ports::{AuthRepository, AuthService},
};
use crate::{
    domain::auth::credentials::{Credentials, CredentialsError},
    outbound::telemetry::spawn_blocking_with_tracing,
};

use anyhow::Context;
use async_trait::async_trait;
use secrecy::{ExposeSecret, Secret};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct BlogAuth<R>
where
    R: AuthRepository,
{
    pub repo: Arc<R>,
}

impl<R> BlogAuth<R>
where
    R: AuthRepository,
{
    pub fn new(repo: Arc<R>) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl<R> AuthService for BlogAuth<R>
where
    R: AuthRepository,
{
    async fn validate_credentials(
        &self,
        credentials: Credentials,
    ) -> Result<uuid::Uuid, CredentialsError> {
        tracing::Span::current()
            .record("username", tracing::field::display(credentials.username()));
        let stored_credentials = self
            .repo
            .get_stored_credentials(credentials.username())
            .await?;

        let user_id = credentials.validate(stored_credentials).await?;
        tracing::Span::current().record("user_id", tracing::field::display(&user_id));
        Ok(user_id)
    }

    async fn get_username(&self, user_id: uuid::Uuid) -> Result<String, CredentialsError> {
        self.repo
            .get_username(user_id)
            .await
            .map_err(CredentialsError::Unexpected)
    }

    async fn change_password(
        &self,
        user_id: uuid::Uuid,
        password: Secret<String>,
    ) -> Result<(), CredentialsError> {
        let password_hash = spawn_blocking_with_tracing(move || compute_password_hash(password))
            .await
            .context("Failed to compute a new password hash")
            .map_err(CredentialsError::Unexpected)??;

        let credentials =
            StoredCredentials::new(user_id, password_hash.expose_secret().to_string());

        self.repo
            .change_password(credentials)
            .await
            .map_err(CredentialsError::Unexpected)
    }
}
