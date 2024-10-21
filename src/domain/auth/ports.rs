use async_trait::async_trait;

use crate::domain::auth::credentials::{Credentials, CredentialsError, StoredCredentials};

#[async_trait]
pub trait AuthRepository: Clone + Send + Sync + 'static {
    async fn get_stored_credentials(
        &self,
        username: &str,
    ) -> Result<Option<StoredCredentials>, CredentialsError>;
}

#[async_trait]
pub trait AuthService: Clone + Send + Sync + 'static {
    async fn validate_credentials(&self, credentials: Credentials) -> Result<(), CredentialsError>;
}
