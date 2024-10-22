use async_trait::async_trait;
use secrecy::Secret;

use crate::domain::auth::credentials::{Credentials, CredentialsError, StoredCredentials};

#[async_trait]
pub trait AuthRepository: Clone + Send + Sync + 'static {
    async fn get_stored_credentials(
        &self,
        username: &str,
    ) -> Result<Option<StoredCredentials>, CredentialsError>;
    async fn get_username(&self, user_id: uuid::Uuid) -> Result<String, anyhow::Error>;
    async fn change_password(&self, credentials: StoredCredentials) -> Result<(), anyhow::Error>;
}

#[async_trait]
pub trait AuthService: Clone + Send + Sync + 'static {
    async fn validate_credentials(
        &self,
        credentials: Credentials,
    ) -> Result<uuid::Uuid, CredentialsError>;
    async fn get_username(&self, user_id: uuid::Uuid) -> Result<String, CredentialsError>;
    async fn change_password(
        &self,
        user_id: uuid::Uuid,
        password: Secret<String>,
    ) -> Result<(), CredentialsError>;
}
