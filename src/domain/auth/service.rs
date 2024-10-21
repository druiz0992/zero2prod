use async_trait::async_trait;

use super::ports::{AuthRepository, AuthService};
use crate::domain::auth::credentials::{Credentials, CredentialsError};

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
    async fn validate_credentials(&self, credentials: Credentials) -> Result<(), CredentialsError> {
        tracing::Span::current()
            .record("username", tracing::field::display(credentials.username()));
        let stored_credentials = self
            .repo
            .get_stored_credentials(credentials.username())
            .await?;

        let user_id = credentials.validate(stored_credentials).await?;
        tracing::Span::current().record("user_id", tracing::field::display(&user_id));
        Ok(())
    }
}
